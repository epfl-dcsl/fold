#import "@preview/fletcher:0.5.8" as fletcher: diagram, node, edge
#import "code.typ": code

#align(center + horizon)[
  #text(size: 4em)[Fold]
  #block(height: -4em)
  #text(size: 1.5em)[A Dynamic linker framework]

  #text(size: 1.2em)[
    Ludovic Mermod - Noé Terrier
    #block(height: -1.8em)
    Semester Research Project, July 2025
    #block(height: -1.8em)
    Data Center Systems Laboratory, EPFL
  ]
]


#pagebreak()


#set page(header: align(right, [Ludovic Mermod & Noé Terrier]))
#set text(size: 12pt)
#set par(justify: true)
#set quote(block: true, quotes: true)
#show quote: it => align(center, it)

#counter(page).update(1)


= Abstract

Fold is a framework to create Rust-based (dynamic) loader, offering simple tools to design and implement new loaders. It provides a default modularized System V ABI loader, on top of which one can add incremental augments for custom purposes.

#outline()

#pagebreak()

#set page(numbering: "1")

= Motivation

When looking at the System landscape, it is clear that research is far ahead of actual implementations, as incorporating new technologies require to either merge them into the Linux Kernel or write a whole new OS. Both are very time consuming and while the latter is more likely to succeed, it would most probably never get actually used.

#quote(attribution: [Rob Pike@pike-rant])[Systems Software Research is Irrelevant]

An interesting observation we can make on the inner working of Linux is that all processes, up to init itself, are launched by the system's dynamic loader@os-narrow-waist. This could be taken advantage of as changing the dynamic loader would allow to execute user-defined code at the start of all processes. Furthermore, an ELF binary can specify the path of its loader, allowing it to pick an appropriate loader.

However, existing loaders like GNU's or MUSL's are very complex pieces of codes, intertwined with their respective standard library, making them hard to tweak. For example, when launching a process with GNU's loader, it first starts by linking itself with `libc`, and vice-versa as they both depend on each other, before finally linking the actual executable.

= System V and ELF recap

= Fold Design

The idea behind Fold's design is similar to assembly lines: an object called the "manifold" is ran through several successive "modules", each of them modifying the manifold. Modules do not communicate between them, except though the manifold itself.

The manifold structure shown in #ref(<manifold-src>) contains arrays of ELF objects, sections and segments, as well as a `ShareMap`. The latter is a structure able to store any datatype and is used to implement inter-module communication: a module can insert data into the map that can then be fetched and used by the following modules.

#code("fold/src/manifold.rs", ident: "Manifold", type: "struct", caption: "Manifold structure")<manifold-src>

For example, let's take a look at the first steps of the default System V module chain (#ref(<fig-sysv-chain>)). First, the manifold is initialized with the ELF file of the binary to load. It then goes through the first module which computes and load the dependencies of the executable recursively, yielding a manifold with the initial ELF file and all the dependencies ELFs. This is then passed over to the `Load` module which sets up the address space and load all the segments, both from the initial file and the dependencies, at their respective addresses.

#figure(
  diagram(
    node-stroke: .1em,
    spacing: 5em,
    edge((-1, 0), "r", "-|>", `ELF file`, label-pos: 0, label-side: center),
    node((0, 0), `Collect`, radius: 2em),
    edge(align(center, raw("ELF file\ndeps list")), "-|>"),
    node((1, 0), `Load`, radius: 2em),
    edge(align(center, raw("ELF file\nELF deps")), "-|>"),
    node((2, 0), `...`, radius: 2em),
    edge("-|>", align(center, raw("ELF file\nELF deps\n...")), label-side: left),
    node((3, 0), `Start`, radius: 2em, extrude: (-2.5, 0)),
  ),
  caption: [System V chain],
)<fig-sysv-chain>

= System V Modules

== Collector

The `SHT_DYNAMIC` sections of the ELF target define the name of the dependencies the object depends on. The role of this phase is to collect in a tree manner the names of dependencies from all objects.

An interesting modification at this stage is to replace a dependency name to another. For example, using `MUSL` version of `libc` instead of the one from `gnu`.

== Loader

This phase is responsible to load `PT_LOAD` tagged segments to they right places, from both the ELF target and its dependencies. Size, offset and mapped address are specified inside the segment program header (`p_vaddr`, `p_offset` and `p_memsz`).

The pase start asking to Linux to map each segment to `p_vaddr`.

In case where the address is 0, it probably means that the ELF was compiled as a Position Independent Executable. In such case, Linux will choose the base loading address itself. The phase can forward the value to the next phases which may need it by storing it inside the shared map.

Once we have a mapping for the segment, the phase copy the content from the binary to the mapping, offset-ed by `p_offset`.

remaining space in the mapping, which can be identify by substracting `p_memsize`, the size of the segment in memory, to `p_filesize`, the size of the segment in the ELF. This residual space may be used for zeroed data used by the program.

We don't care about memory protection here as we may want to be able to write to segment latter.

== Thread local storage
== Relocation

A relocation is a statically planned modification of the content of the loaded segments, resolved during linking stage with additional informations, as data from dependencies or even result of code execution. There is a important variety of relocation types, each with its own computation.

This phase process sections with tag SHT_RELA. Each section point to an array of Rela entries, storing all relocations of the ELF.

== Protect
== Init array
== Fini array
== Start

= Example implementations

== Seccomp
== Trampoline

#bibliography("report.bib")
