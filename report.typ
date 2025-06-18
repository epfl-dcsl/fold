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
#counter(page).update(1)

= Abstract

#outline()

#pagebreak()

#set page(numbering: "1")

= Overall description

#code("fold/src/main.rs", ident: "entry", caption: "Linker entrypoint")

= System V Modules

== Collector
== Loader
== Thread local storage
== Protect
== Init array
== Fini array
== Start

= Example implementations

== Seccomp
== Trampoline

