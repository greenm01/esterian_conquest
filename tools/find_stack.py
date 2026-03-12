import sys

with open('/tmp/ecmaint-debug-token/MEMDUMP.BIN', 'rb') as f:
    mem = f.read()

print(f"Memory dump size: {len(mem)} bytes")

# We know the breakpoint is at linear address 0x31804 (2814:96c4 or 3159:0274)
# The function is either FAR or NEAR.
# If FAR, stack has: [Return IP] [Return CS]
# If NEAR, stack has: [Return IP]
# We suspect caller is around 2000:7200 -> linear 2814:7200 = 0x2F340
# Wait! In Ghidra, 2000:7200 is segment 2000. In memory it's 2814:7200.
# So Return CS might be 0x2814, or some other segment.
# In the previous trace, we saw Return CS=0x3000 (Ghidra) -> memory 0x3814.
# Or if it's an overlay, it might be different.

# Let's just find the SS register by searching for F926!
# We know the instruction is `les di, [bp+06] ss:[F92C]`
# This was resolved by DOSBox-X. It means `SS * 16 + F92C` is a valid address.
# Let's search memory for pointers to our function!
# Wait, let's just dump all values that look like CS:IP from the top of all 64k segments around F928

sp_target = 0xF928

candidates = []
# SS is likely between 0x1000 and 0x9000
for ss in range(0x1000, 0x9000):
    linear = (ss * 16) + sp_target
    if linear + 4 <= len(mem):
        # Read what's at the stack
        ret_ip = int.from_bytes(mem[linear:linear+2], 'little')
        ret_cs = int.from_bytes(mem[linear+2:linear+4], 'little')
        
        # Check if ret_cs is a valid code segment
        if 0x1000 <= ret_cs <= 0x9000:
            candidates.append((ss, ret_ip, ret_cs, linear))

print(f"Found {len(candidates)} potential SS values where [SS:F928] looks like a FAR return address.")
for ss, rip, rcs, lin in candidates:
    # Filter for realistic CS
    if abs(rcs - 0x2814) < 0x2000:
        print(f"Match: SS={hex(ss)} Linear={hex(lin)} -> Return: {hex(rcs)}:{hex(rip)}")
