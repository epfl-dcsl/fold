#import "@preview/fletcher:0.5.8" as fletcher: diagram, edge, node
#import fletcher.shapes: brace, bracket
#import "../common/code.typ": code

#let appendix(body) = {
  set heading(numbering: "A", supplement: [Appendix])
  counter(heading).update(0)
  body
}

#align(center + top)[
  #grid(
    columns: auto,
    row-gutter: 1em,
    text(size: 3em)[Thread Local Storage in the Fold Framework],

    pad(y: 1.5em, [by]),

    text(size: 1.5em)[Lucie Mermod],
  )]

#align(
  center + horizon,
  pad(x: 30pt)[
    = Abstract

    Fold is a framework to create Rust-based (dynamic) linkers, offering simple tools to design and implement new linkers. This work aims to implement fully featured Thread Local Storage support which was lacking in Fold's first implementation.
  ],
)

#align(center + bottom)[
  #text(size: 10pt)[
    #grid(
      columns: (auto, auto),
      column-gutter: 1em,
      row-gutter: 0.4em,
      align: left,
      [Date:], [December 2025],
    )]
]

#pagebreak()

#set page(header: align(right, [Lucie Mermod]))
#set heading(numbering: "1.1")
#set text(size: 12pt)
#set par(justify: true)
#set quote(block: true, quotes: true)
#show quote: it => align(center, it)

#counter(page).update(1)

#outline()

#pagebreak()

#set page(numbering: "1")

= Motivation<sect-motivation>

Most of nowadays applications rely on multi-threading to efficiently achieve their goal, e.g. by processing data through streaming pipelines or by dispatching different tasks accross threads. This means that for any loader aiming at being usable in a real environment, it must provide an API adequate for multi-threading.

Luckily, there exists since 2002 a specification describing how ELF should store thread-related data, and how loaders need to handle it @tls-spec. This document concerns many architectures other than `x86_64`, so I will summarize the relevant parts @sect-bg-tls.

= Background<sect-bg>

Before diving into how Fold implements Thread Local Storage (TLS), we first need to take a look at what it is exactly, as well as how Musl's standard library @musl interacts with it. I would like to thank Chao-tic @chaotic-blog, MaskRay @maskray-blog and the Android developpers @android-blog for their blogs which helped a lot in understanding how TLS works.

== Thread Local Storage<sect-bg-tls>

Thread Local Storage (TLS in short) is the in-memory data structure holding all the data which is unique to a thread, i.e. the Thread Control Block (TCB) and all static variables denoted with `__thread_local`, whether they are defined in the executable's ELF or in any of the shared objects it depends on.

Each ELF object can contain at most one segment marked with `PT_TLS`, indicating that this segment should be loaded as part of the TLS structure. Such segment is called a TLS module and is characterized, as all segments, with its offset in the file, size in the file, size in the memory, etc. If the size in memory is larger than the size in the file, the remaining bytes should be zeroed out when the module is loaded. This segment is usually split into two sections, named `.tdata` and `.tbss`, analogous to the `.data` and `.bss` sections.

The TLS specification @tls-spec describes two memory layouts to hold the TLS, but we will focus on the second variant as it is the one used in x86_64. The figure below gives an overall view of the structure:

#figure(
  image("../common/tls-layout.png"),
  caption: [TLS in-memory layout in x86_64],
)

There are four main components:

- Thread control block (TCB): holds thread-specific information such as the Thread Identifier (TID), stack canary, etc. It also contains a pointer to the DTV (below).
- Static modules (at $"tlsoffset"_x$): TLS modules which are loaded at load-time or during thread creation.
- Dynamic modules: TLS modules which are loaded only when the thread attempts to access memory within them.
- Dynamic Thread Vector (DTV): stores a pointer to each loaded modules, as well as a generation number used when library are dynamically loaded at run-time (using `dlopen`).

When creating the TLS, the loader must identify all the TLS modules across the executable and its dependencies, and assign a Module ID to each of them. IDs starts at 1, which is reserved for the executable's TLS module, if any. Other modules can technically be assigned with any IDs, although for practical reasons the loader will usually simply count from 2.

There are two reasons for the existence of dynamic modules. First, it makes the implementation of `dlopen()` and related functions much easier and safer as they are allowed to place the TLS modules of the new objects anywhere in the address space, and not specifically before the existing modules (this could result in clashing with another existing memory mapping). Also, some modules could be ignore by some threads and loading them would be a waste of time and memory, hence the posssibilty to defer this allocation when the module is actuall accessed (see @naive-tls-get-addr in General Dynamic accesses below).

When a thread wants to access a variable from a TLS module, it can proceed with different methods, by decreasing genericity:

- *General Dynamic*: Using different relocations, the loader will allocate a struct in the GOT to store the Module ID and Offset of variable within the module, which the runtime will then pass to the `__tls_get_addr(uintptr_t[2])` function which will return the actual address of the variable. This function should be provided by the loader itself. A naive implementation would look like this:

  #figure(
    ```C
    void* __tls_get_addr(uintptr_t[2] req) {
      // Allocate the module with id req[0]
      if (tcb->dtv[req[0]] == NULL) {
        allocate_tls_module(req[0]);
      }

      return tcb->dtv[req[0]] + req[1];
    }
    ```,
    caption: [simple `__tls_get_addr` implementation],
    kind: "code",
    supplement: "Code snippet",
  )<naive-tls-get-addr>

- *Local Dynamic*: If the code accessing the variable is from the same object as the TLS module requested, then it may call the `__tls_get_addr()` function and pass an offset of `0` to get the address of the TLS module, and the compiler can add the offset itself since it would be known at compile time. This method is only interseting if the program accesses multiple thread-local variables, as it reduces to one the number of calls to `__tls_get_addr()`.
- If the module is statically loaded, then the program can access the variable by offsetting the thread pointer. To do so, it can either:
  - *Initial-Exec*: Request the loader to generate a GOT entry to store the offset
  - *Local-Exec*: Produce a relocation asking the loader to directly write the offset into the code.

Note that the specification states that Local-Exec cannot be used from a shared object, but I did not understand why this restriction exists.

It is unclear who, between the programmer, compiler or loader, decides which module should be statically loaded. While a `DF_STATIC_TLS` flag exists in the Dynamic Table, I found that it is not produced by compilers and cannot be the sole decider whether the modules should be static as this decision also depends on how other objects access it (which is obviously not known when the compiler generates the shared object). From my understanding, any module may be static except for those accessed through the `R_X86_64_TPOFF32` and `R_X86_64_TPOFF64` (used in Initial-Exec and Local-Exec) which have to be static. Those considerations taken into account, the loader is free to choose whether to allocate dynamically or not the other modules.

== Musl's implementation<sect-bg-musl>

Knowing the fundamentals of Thread Local Storage, let's now take a look at the implementation details of Musl.

=== Module allocation<sect-bg-musl-alloc>

The first observation to be made is that Musl provides itself an implementation for `__tls_get_addr` (@musl-tls-get-addr) which simply indexes the `dtv` array, meaning that all TLS modules are statically allocated. This greatly simplifies to process of allocating the TLS modules, but also increases the overhead of loading the ELF and creating new threads.

#code(
  "musl/src/thread/__tls_get_addr.c",
  ident: "__tls_get_addr",
  type: "void\s*\*",
  caption: [Musl's implementation of `__tls_get_addr`],
  lang: "C",
)<musl-tls-get-addr>

=== Thread Control Block<sect-bg-musl-tcb>

On top of the Thread Control Block specified in the ABI (todo: ref wanted), Musl augments it with quite a lot of fields, as shown in @musl-tcb of @app-musl-tcb. Luckily, a lot of these fields are handled by Musl's runtime and should be initialized to `0`; and the function `__init_tp` (@init-tp) shows us which fields should be initialized and how.

#code("musl/src/env/__init_tls.c", ident: "__init_tp", type: "int", caption: [`__init_tp`])<init-tp>

=== The `libc` object<sect-bg-musl-libc>

When inspecting the `__init_tp` function (@init-tp), we encounter a rather peculiar object called `libc` (@libc). It is a global variable in the Musl library that stores general informations about the runtime. Here, we will focus on the thread-related fields which are for the most part quite simple.

- `can_do_threads`: Whether the TLS was correctly initialized. Needs to be set to `1` in order for the `thrd_create` function to succeed.
- `threaded`: Handled by Musl upon first call of `thrd_create`, indicates whether the library has initialized its thread-related components.
- `threads_minus_1`: Number of threads running, minus 1.
- `tls_head`: A linked list of `tls_module` (@musl-tls-module) objects. This list is used to create the TLS region when an new thread is insantiated. It must be ordered according to the Module IDs.
- `tls_size`: Total size in bytes of the TLS region, ergo size of the TCB, all the modules (remember that they are all statically loaded) and the padding, if needed.
- `tls_align`: The alignement restriction for the TLS region, which is the maximum of all the `align` fields of the TLS modules and the alignement of the TCB itself.
- `tls_cnt`: Number of TLS modules.

#code("musl/src/internal/libc.h", type: "struct", ident: "__libc", caption: [The `libc` object])<libc>


#code(
  "musl/src/internal/libc.h",
  type: "struct",
  ident: "tls_module",
  caption: [The `tls_module` object],
)<musl-tls-module>

Similarly, there is a `__sysinfo` object where the loader should store the value of the `AT_SYSINFO` from the auxiliary vector, but this is ignored in 64 bits mode @elf-auxv.

=== Observations<sect-bg-musl-observations>

It is interesting to note that when Musl creates a thread, it sets up the TLS region at the top of the stack. While this may be peculiar at first, it makes sense as the TLS region and the stack are the only memory region specific to a thread, the text and data segment are shared with all the other threads of the process. However, this could be a security concern as it makes the reference stack canary stored in the TCB may be accessed by overflowing the stack.


#figure(
  diagram(
    spacing: 0pt,
    node-shape: rect,
    node-stroke: 1pt,

    node((4, -3), width: 25mm, height: 2mm, stroke: none),
    node((4, -2), width: 25mm, height: 2mm, stroke: none),
    node((4, -1), width: 25mm, height: 2mm, stroke: none),
    node((4, 0), width: 25mm, height: 4mm, stroke: none),


    node((0, 1), [... stack]),

    {
      let color = blue.lighten(50%)
      node((1, 1), [gen], fill: color, name: <gen>)
      node((2, 1), [PM1], fill: color, name: <p1>)
      node((3, 1), [PM2], fill: color, name: <p2>)
      node(enclose: ((1, 1), (3, 1)), shape: brace.with(label: [DTV]), inset: 2pt)
    },


    {
      let color = orange.lighten(50%)
      node((4, 1), [M2], fill: color, width: 25mm, name: <m2>)
      node((4, 2), width: 25mm, height: 5mm, stroke: none)
      node(enclose: ((4, 2), (5, 1)), shape: bracket.with(label: [$"tlsoffset"_2$]), inset: 2pt)

      node((5, 1), [M1], fill: color, width: 25mm, name: <m1>)
      node(enclose: ((5, 1), (5, 1)), shape: bracket.with(label: [$"tlsoffset"_1$]), inset: 2pt)
    },


    {
      let color = green.lighten(50%)
      node((6, 1), [TCB], fill: color, width: 15mm, name: <tcb>)
    },

    edge(<tcb>, "uuuu", "lllll", "dddd", "-|>", stroke: 1pt),
    edge(<p1>, "uuu", "rrr", "ddd", "-|>", stroke: 1pt),
    edge(<p2>, "uu", "r", "dd", "-|>", stroke: 1pt),
  ),
  caption: [TLS region in Musl],
)

= Implementation<sect-impl>

To fully handle TLS in the Fold architecture, I added four modules. The first one is not specific to TLS nor to SystemV (hence placed in `fold/src/`) while the three others form a chain to identify and process all TLS-related info in the ELF.

== Musl<sect-impl-musl>

This first module has a simple purpose: locating the objects discussed in @sect-bg-musl-libc (`__libc` and `__sysinfo`) as they are needed when setting up the TCB (see @init-tp). Since these objects need to be written to, the module cannot simply clone them and put them in the shared memory, neither can it store mutable references. Hence, the module stores a small struct indicating in which segment and at which offset the object is stored (see @musl-object-idx).

#code(
  "fold/src/musl.rs",
  type: "struct",
  ident: "MuslObjectIdx",
  caption: [Structure used to store the location of a Musl object],
)<musl-object-idx>

This structure then exposes two functions `get` and `get_mut` that return (mutable) references to the underlying object. This way, any module that wants to access a Musl Object can do so by first retrieving and cloning the `MuslObjectIdx` from the shared map, and then using it to borrow part of the manifold's struct. Under the hood, these functions uses the `zerocopy` crate@zerocopy which allows to safely convert a `&[u8]` into a `&T` or a `&mut [u8]` into `&mut T`, and vice-versa.

== Collection<sect-impl-collect>

Now let's look into the TLS-related modules. The role of `TlsCollector` is to identify all the TLS modules and generate their metadata: Module ID and tlsoffset. For the former, it checks whether the object containing the module is the initial ELF (using the new `INITIAL_ELF_KEY` shared map entry) or uses a counter to generate new IDs otherwise. The formula to compute the offset of a module is shown in @fig-tlsoffset-compute (TLS specification §3.4.6@tls-spec). The module then stores a `Vec` containing all the TLS modules (@code-tlsmodule) into the manifold's shared map, as well as the individual elements in their respective object's shared map.

#code(
  "fold/src/sysv/tls/collection.rs",
  type: "struct",
  ident: "TlsModule",
  caption: [Output of the `TlsCollector` module],
)<code-tlsmodule>

#figure(
  [
    $
      cases(
        "tlsoffset"_1="round"("tls_size"_1,"tls_align"_1),
        "tlsoffset"_(n+1)="round"("tlsoffset"_n+"tls_size"_(n+1),
          "tls_align"_(n+1))
      )
    $
    with $"round"(x,y) eq.def y dot ceil(x/y)$
  ],

  caption: [Computation of `tlsoffset`],
)<fig-tlsoffset-compute>

== Allocation<sect-impl-alloc>

TODO: maybe split into TlsAllocation and TlsMusl

This module is the heart of the implementation of TLS. Its role is to allocate and initialize the memory region that will store the TCB, DTV and all the modules (following Musl's implementation, all modules are statically allocated). It begins by allocating a new memory region using `mmap`, then fills it according to @fig-main-tls-region. It also updates the `libc` object with all the data related to TLS; the `tls_head` list is constructed by this module and stored in the shared map to be exposed to the runtime.

#figure(
  diagram(
    spacing: 0pt,
    node-shape: rect,
    node-stroke: 1pt,

    node((4, -3), width: 25mm, height: 2mm, stroke: none),
    node((4, -2), width: 25mm, height: 2mm, stroke: none),
    node((4, -1), width: 25mm, height: 2mm, stroke: none),
    node((4, 0), width: 25mm, height: 4mm, stroke: none),


    {
      let color = blue.lighten(50%)
      node((0, 1), [gen], fill: color, name: <gen>)
      node((1, 1), [PM1], fill: color, name: <p1>)
      node((2, 1), [PM2], fill: color, name: <p2>)
      node(enclose: ((0, 1), (2, 1)), shape: brace.with(label: [DTV]), inset: 2pt)
    },

    node((3, 1), [pad]),

    {
      let color = orange.lighten(50%)
      node((4, 1), [M2], fill: color, width: 25mm, name: <m2>)
      node((4, 2), width: 25mm, height: 5mm, stroke: none)
      node(enclose: ((4, 2), (5, 1)), shape: bracket.with(label: [$"tlsoffset"_2$]), inset: 2pt)

      node((5, 1), [M1], fill: color, width: 25mm, name: <m1>)
      node(enclose: ((5, 1), (5, 1)), shape: bracket.with(label: [$"tlsoffset"_1$]), inset: 2pt)
    },


    {
      let color = green.lighten(50%)
      node((6, 1), [TCB], fill: color, width: 15mm, name: <tcb>)
    },

    edge(<tcb>, "uuuu", "llllll", "dddd", "-|>", stroke: 1pt),
    edge(<p1>, "uuu", "rrrr", "ddd", "-|>", stroke: 1pt),
    edge(<p2>, "uu", "rr", "dd", "-|>", stroke: 1pt),
  ),
  caption: [Main thread's TLS region in Fold],
)<fig-main-tls-region>

Particular care must be taken when computing the addresses, as each components have their own alignment restriction: the TCB and DTV are aligned on 8 bytes, and each module may request a specific alignment. To cover this, an optional padding region is added between the DTV and the modules (see @fig-main-tls-region), such that the DTV is at the start of a page, and the TCB can be aligned on the maximum alignment requirement of the TCB and all the modules. After that, the `tlsoffset` formula (@fig-tlsoffset-compute) ensures that all the modules are properly aligned.

== Relocation<sect-impl-reloc>

This last modules processes all the TLS-related relocations found in the objects. It is separated from `SysvReloc`, but it may be interesting to merge both for performance reasons; using separate modules means that the relocations will be iterated over twice.

The relocations handled by `TlsRelocator` are:

- `R_X86_64_TPOFF{32,64}`: Offset of the symbol relative to the thread-pointer, i.e. `tlsoffset + sym.value`.
- `R_X86_64_DTPMOD64`: Module ID of the TLS module containing the symbol.
- `R_X86_64_DTPOFF{32,64}`: Offset of the symbol within its TLS module, i.e. `sym.value`.
- `R_X86_64_GOTTPOFF`, `R_X86_64_TLSGD` and `R_X86_64_TLSLD` are left unimplemented for now as they require proper handling of the GOT, which is not yet implemented in Fold.

= State of the project

#pagebreak()

#show bibliography: set heading(numbering: "1.1")
#bibliography("report.bib", title: [References])

#show: appendix

#pagebreak()

= Musl's TCB<app-musl-tcb>

#text(
  [#code(
    "musl/src/internal/pthread_impl.h",
    ident: "pthread",
    type: "struct",
    caption: [Musl's Thread Control Block],
  )<musl-tcb>],
  size: 11.5pt,
)

