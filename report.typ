
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


#let snippet(file, line, length) = {
  let code = read(file).split("\n")

  let i = 0
  while code.at(line).starts-with(" " * i) {
    i += 1
  }
  i -= 1

  return raw(
    code.slice(line, none, count: length).map(s => if s.len() > i { s.slice(i, none) } else { s }).join("\n"),
    lang: "rust",
  )
}

#let count(str, char) = {
  let count = 0
  for i in range(0, str.len()) {
    if str.at(i) == char {
      count += 1
    }
  }

  return count
}

#let extract-code(file, ident, type: "\w+") = {
  let code = read(file).split("\n")

  let i = 0
  while i < code.len() and code.at(i).match(regex(type + "\s+" + ident + "\b")) == none { i += 1 }
  let start = i
  assert(i != code.len(), message: "Identifier " + ident + " not found in file " + file)

  let brackets = 0
  while start < code.len() {
    brackets += count(code.at(i), "{")
    brackets -= count(code.at(i), "}")

    i += 1
    if brackets <= 0 { break }
  }

  return snippet(file, start, i - start)
}


#pagebreak()

#set page(header: align(right, [Ludovic Mermod & Noé Terrier]))
#set text(size: 12pt)

= Abstract

#outline()

#pagebreak()

#set page(numbering: "1")
#counter(page).update(1)

= Overall description

#extract-code("fold/src/main.rs", "entry", type: "fn")

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

