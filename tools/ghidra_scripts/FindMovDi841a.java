
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.scalar.Scalar;

public class FindMovDi841a extends GhidraScript {
    @Override
    public void run() throws Exception {
        InstructionIterator iter = currentProgram.getListing().getInstructions(currentProgram.getMinAddress(), true);
        while (iter.hasNext()) {
            Instruction ins = iter.next();
            if (ins.getMnemonicString().equals("MOV")) {
                Object[] opers = ins.getOpObjects(1);
                if (opers.length > 0 && opers[0] instanceof Scalar) {
                    long val = ((Scalar)opers[0]).getValue();
                    if (val == 0x841a || val == 0x841b) {
                        println("Found MOV DI, " + val + " at " + ins.getAddress() + ": " + ins.toString());
                    }
                }
            }
        }
    }
}
