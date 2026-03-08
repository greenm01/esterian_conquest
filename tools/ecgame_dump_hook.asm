bits 16
org 0x220

start:
    push cs
    pop ds
    push cs
    pop es

    mov dx, filename
    xor cx, cx
    mov ah, 0x3c
    int 0x21
    jc fail

    xchg bx, ax
    xor ax, ax
    mov [es:seg_cur], ax

dump_loop:
    mov ax, [es:seg_cur]
    cmp ax, 0xa000
    jae close_file

    push ds
    mov ds, ax
    xor dx, dx
    mov cx, 0x8000
    mov ah, 0x40
    int 0x21
    pop ds
    jc close_file

    add word [es:seg_cur], 0x0800
    jmp dump_loop

close_file:
    mov ah, 0x3e
    int 0x21

fail:
    mov ax, 0x4c00
    int 0x21

seg_cur:
    dw 0

filename:
    db 'ECDUMP.BIN', 0
