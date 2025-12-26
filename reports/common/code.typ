

#let count(str, char) = {
  let count = 0
  for i in range(0, str.len()) {
    if str.at(i) == char {
      count += 1
    }
  }

  return count
}

#let fix-identation(code) = {
  let i = 0
  while code.at(0).starts-with(" " * i) {
    i += 1
  }
  i -= 1

  return code.map(s => if s.len() > i { s.slice(i, none) } else { s }).join("\n")
}

#let extract-identifier(code, ident, type: "\w+") = {
  let i = 0
  while i < code.len() and code.at(i).match(regex(type + "\s*" + ident + "\b")) == none { i += 1 }
  let start = i
  assert(i != code.len(), message: "Identifier " + ident + " not found")


  let brackets = 0
  while brackets == 0 and i < code.len() {
    brackets += count(code.at(i), "{")
    brackets -= count(code.at(i), "}")

    i += 1
  }

  while i < code.len() {
    brackets += count(code.at(i), "{")
    brackets -= count(code.at(i), "}")

    i += 1
    if brackets <= 0 { break }
  }

  return fix-identation(code.slice(start, i))
}

#let code(file, ident: none, type: none, from: none, to: none, lang: "rust", caption: none) = {
  let code = read("../../" + file).split("\n")

  assert(
    ident != none or (from != none and to != none),
    message: "Invalid parameters, either ident should be set or from and to.",
  )

  let content
  if ident != none {
    content = extract-identifier(code, ident, type: type)
  } else if from != none and to != none {
    content = fix-identation(code.slice(form, to))
  }

  content = raw(content, align: left, lang: lang, tab-size: 4)

  if caption != none {
    content = figure(align(left, content), caption: caption, kind: "code", supplement: "Code snippet")
  }

  return content
}
