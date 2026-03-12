
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;

public class FindCall4f4c extends GhidraScript {
    @Override
    public void run() throws Exception {
        InstructionIterator iter = currentProgram.getListing().getInstructions(currentProgram.getMinAddress(), true);
        while (iter.hasNext()) {
            Instruction ins = iter.next();
            String mnemonic = ins.getMnemonicString();
            if (mnemonic.equals("CALLF") || mnemonic.equals("CALL")) {
                Object[] opers = ins.getOpObjects(0);
                if (opers.length > 0 && opers[0].toString().contains("3000:4f4c")) {
                    println("Call to 3000:4f4c at " + ins.getAddress() + ":");
                    // Try to find the preceding MOV DI, XXXX
                    Instruction prev = ins.getPrevious();
                    for (int i = 0; i < 5 && prev != null; i++) {
                        println("  " + prev.getAddress() + ": " + prev.toString());
                        prev = prev.getPrevious();
                    }
                }
            }
        }
    }
}
