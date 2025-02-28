; NASM's syntax for exporting global is a bit funky,
; but everything has a purpose.

        ; here we declare `msg` as a global, and we say it's of type
        ; `data` (as opposed to `function`). We also have to specify
        ; its length, which is `end-start`.
        global msg:data msg.end-msg

        ; we only have a data section for this file
        section .data

; `msg` is a label - which makes sense, a label is just a name for
; a place in memory! we've seen the `db` syntax earlier, same here
msg: db "hi there", 10
; this here is a local label - it belongs to `msg` and can be referred
; using `msg.end`. This is how we compute the length of `msg`!
 .end:
