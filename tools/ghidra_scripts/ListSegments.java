
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.mem.MemoryBlock;

public class ListSegments extends GhidraScript {
    @Override
    public void run() throws Exception {
        for (MemoryBlock block : currentProgram.getMemory().getBlocks()) {
            println("Block: " + block.getName() + " [" + block.getStart() + " - " + block.getEnd() + "]");
        }
    }
}
