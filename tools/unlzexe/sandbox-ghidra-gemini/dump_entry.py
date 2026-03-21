# Dump entry point using Jython
# @category Sandbox

from ghidra.program.model.address import Address

def run():
    entry_sym = currentProgram.getSymbolTable().getPrimarySymbol("entry")
    if not entry_sym:
        print("No 'entry' symbol found!")
        return
    entry_addr = entry_sym.getAddress()
    print("Entry Point Address: {}".format(entry_addr))
    
    listing = currentProgram.getListing()
    ins_iter = listing.getInstructions(entry_addr, True)
    
    print("Disassembly starting at {}:".format(entry_addr))
    count = 0
    while ins_iter.hasNext() and count < 50:
        ins = ins_iter.next()
        print("  {}: {}".format(ins.getAddress(), ins))
        count += 1

if __name__ == '__main__':
    run()
