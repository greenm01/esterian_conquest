bits 16
org 0x220

start:
    push cs
    pop ds
    mov dx, filename
    xor cx, cx
    mov ah, 0x3c
    int 0x21
    jc done
    xchg bx, ax
    mov dx, message
    mov cx, message_end - message
    mov ah, 0x40
    int 0x21
    mov ah, 0x3e
    int 0x21
done:
    mov ax, 0x4c00
    int 0x21

filename:
    db 'ECHIT.TXT', 0
message:
    db 'hook hit', 13, 10
message_end:
