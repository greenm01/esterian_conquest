00000000  55                push bp
00000001  89E5              mov bp,sp
00000003  81EC0002          sub sp,0x200
00000007  8B6E04            mov bp,[bp+0x4]
0000000A  3680BD2BEF00      cmp byte [ss:di-0x10d5],0x0
00000010  6403E9            fs add bp,cx
00000013  98                cbw
00000014  018B6E04          add [bp+di+0x46e],cx
00000018  36C6852AEF01      mov byte [ss:di-0x10d6],0x1
0000001E  AF                scasw
0000001F  0225              add ah,[di]
00000021  1E                push ds
00000022  57                push di
00000023  9AE2418528        call word 0x2885:word 0x41e2
00000028  BF9B0B            mov di,0xb9b
0000002B  0E                push cs
0000002C  57                push di
0000002D  9A64358528        call word 0x2885:word 0x3564
00000032  A11B35            mov ax,[0x351b]
00000035  8B1E1D35          mov bx,[0x351d]
00000039  8B161F35          mov dx,[0x351f]
0000003D  9A4D116433        call word 0x3364:word 0x114d
00000042  09D0              or ax,dx
00000044  650CBF            gs or al,0xbf
00000047  C6                db 0xc6
00000048  0B0E579A          or cx,[0x9a57]
0000004C  64358528          fs xor ax,0x2885
00000050  DB0A              fisttp dword [bp+si]
00000052  BFCE0B            mov di,0xbce
00000055  0E                push cs
