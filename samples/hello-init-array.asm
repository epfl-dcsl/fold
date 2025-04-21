; in `hello-init-array.asm`

        global _start

        section .init_array

dq init_1
dq init_2

        section .text

init_1:   mov rdi, 1      ; stdout fd
        mov rsi, ms_init_1
        mov rdx, 23      ; 22 chars + newline
        mov rax, 1      ; write syscall
        syscall
        
        ret

init_2:   mov rdi, 1      ; stdout fd
        mov rsi, ms_init_2
        mov rdx, 23      ; 22 chars + newline
        mov rax, 1      ; write syscall
        syscall
        
        ret

_start: mov rdi, 1      ; stdout fd
        mov rsi, msg
        mov rdx, 9      ; 8 chars + newline
        mov rax, 1      ; write syscall
        syscall

        xor rdi, rdi    ; return code 0
        mov rax, 60     ; exit syscall
        syscall
        
        section .data

ms_init_1:    db "hi there (from init_1)", 10
ms_init_2:    db "hi there (from init_2)", 10
msg:    db "hi there", 10
