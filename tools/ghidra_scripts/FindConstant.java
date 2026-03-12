
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.scalar.Scalar;

public class FindConstant extends GhidraScript {
    @Override
    public void run() throws Exception {
        InstructionIterator iter = currentProgram.getListing().getInstructions(currentProgram.getMinAddress(), true);
        while (iter.hasNext()) {
            Instruction ins = iter.next();
            for (int i = 0; i < ins.getNumOperands(); i++) {
                Object[] opers = ins.getOpObjects(i);
                for (Object op : opers) {
                    if (op instanceof Scalar) {
                        long val = ((Scalar)op).getValue();
                        if (val == 0x841a || val == 0x841b) {
                            println("Found constant at " + ins.getAddress() + ": " + ins.toString());
                        }
                    }
                }
            }
        }
    }
}
