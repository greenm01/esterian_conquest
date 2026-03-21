00000000  50                push ax
00000001  9C                pushf
00000002  1E                push ds
00000003  FA                cli
00000004  8CC8              mov ax,cs
00000006  8ED8              mov ds,ax
00000008  7215              jc 0x1f
0000000A  33FF              xor di,di
0000000C  8BF7              mov si,di
0000000E  B90800            mov cx,0x8
00000011  3331              xor si,[bx+di]
00000013  D1CE              ror si,0x0
00000015  47                inc di
00000016  47                inc di
00000017  E2F8              loop 0x11
00000019  81FEB595          cmp si,0x95b5
0000001D  7427              jz 0x46
0000001F  BAF203            mov dx,0x3f2
00000022  B8FFFF            mov ax,0xffff
00000025  8EC0              mov es,ax
00000027  26803E0E00FD      cmp byte [es:0xe],0xfd
0000002D  7502              jnz 0x31
0000002F  B600              mov dh,0x0
00000031  B80000            mov ax,0x0
00000034  EF                out dx,ax
00000035  8EC0              mov es,ax
00000037  B8C400            mov ax,0xc4
0000003A  26A30800          mov [es:0x8],ax
0000003E  8CC8              mov ax,cs
00000040  26A30A00          mov [es:0xa],ax
00000044  EBFE              jmp 0x44
00000046  8F06B101          pop word [0x1b1]
0000004A  8F06AD01          pop word [0x1ad]
0000004E  8F06AF01          pop word [0x1af]
00000052  BE0E00            mov si,0xe
00000055  BF0300            mov di,0x3
00000058  8B00              mov ax,[bx+si]
0000005A  D1C8              ror ax,0x0
0000005C  32C4              xor al,ah
0000005E  83EE02            sub si,0x2
00000061  8B10              mov dx,[bx+si]
00000063  D1C2              rol dx,0x0
00000065  32C2              xor al,dl
00000067  32C6              xor al,dh
00000069  8885A901          mov [di+0x1a9],al
0000006D  4F                dec di
0000006E  83EE02            sub si,0x2
00000071  79E5              jns 0x58
00000073  800EA90101        or byte [0x1a9],0x1
00000078  8CCD              mov bp,cs
0000007A  2B2EB301          sub bp,[0x1b3]
0000007E  55                push bp
0000007F  B960C3            mov cx,0xc360
00000082  BE0100            mov si,0x1
00000085  8EC5              mov es,bp
00000087  BF0000            mov di,0x0
0000008A  A1A901            mov ax,[0x1a9]
0000008D  8B2EAB01          mov bp,[0x1ab]
00000091  5D                pop bp
00000092  BEB501            mov si,0x1b5
00000095  0336CD01          add si,[0x1cd]
00000099  8B0EBB01          mov cx,[0x1bb]
0000009D  E30D              jcxz 0xac
0000009F  AD                lodsw
000000A0  8BD8              mov bx,ax
000000A2  AD                lodsw
000000A3  03C5              add ax,bp
000000A5  8EC0              mov es,ax
000000A7  26012F            add [es:bx],bp
000000AA  E2F3              loop 0x9f
000000AC  0E                push cs
000000AD  07                pop es
000000AE  BF0000            mov di,0x0
000000B1  8BC7              mov ax,di
000000B3  B95101            mov cx,0x151
000000B6  F3AA              rep stosb
000000B8  012ECB01          add [0x1cb],bp
000000BC  032EC301          add bp,[0x1c3]
000000C0  8ED5              mov ss,bp
000000C2  8B26C501          mov sp,[0x1c5]
000000C6  A1AF01            mov ax,[0x1af]
000000C9  C706C701FBEA      mov word [0x1c7],0xeafb
000000CF  8B16B101          mov dx,[0x1b1]
000000D3  F706AD010002      test word [0x1ad],0x200
000000D9  8B0E0001          mov cx,[0x100]
000000DD  8B1E0301          mov bx,[0x103]
000000E1  8EDA              mov ds,dx
000000E3  8EC2              mov es,dx
000000E5  7520              jnz 0x107
000000E7  EB1F              jmp 0x108
000000E9  0000              add [bx+si],al
000000EB  0000              add [bx+si],al
000000ED  0000              add [bx+si],al
000000EF  50                push ax
000000F0  1900              sbb [bx+si],ax
000000F2  00361C00          add [0x1c],dh
000000F6  0000              add [bx+si],al
000000F8  0000              add [bx+si],al
000000FA  0000              add [bx+si],al
000000FC  0000              add [bx+si],al
000000FE  0000              add [bx+si],al
00000100  0000              add [bx+si],al
00000102  0000              add [bx+si],al
00000104  0000              add [bx+si],al
00000106  00909000          add [bx+si+0x90],dl
0000010A  0000              add [bx+si],al
0000010C  0000              add [bx+si],al
0000010E  0000              add [bx+si],al
00000110  0000              add [bx+si],al
00000112  0000              add [bx+si],al
00000114  0000              add [bx+si],al
00000116  00                db 0x00
