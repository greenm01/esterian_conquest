import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.scalar.Scalar;
import ghidra.program.model.symbol.Reference;
import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class Report5EE4FleetBranch extends GhidraScript {
    private static final String OUT = "artifacts/ghidra/ecmaint-live/5ee4-fleet-branch.txt";

    @Override
    protected void run() throws Exception {
        File outFile = new File(currentProgram.getDomainFile().getProjectLocator().getLocation(), "../../" + OUT);
        File parent = outFile.getCanonicalFile().getParentFile();
        if (!parent.exists() && !parent.mkdirs()) {
            throw new IllegalStateException("failed to create output dir " + parent);
        }

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            writeHeader(out);
            dumpRange(out, "2000:6040", "2000:6368", "fleet_validation_branch");
        }

        println("Report5EE4FleetBranch> wrote " + outFile.getCanonicalPath());
    }

    private void writeHeader(PrintWriter out) {
        Function fn = getFunctionAt(toAddr("2000:5ee4"));
        out.println("Program: " + currentProgram.getName());
        out.println();
        out.println("Function: " + (fn == null ? "<none>" : fn.getName()));
        out.println("Range: 2000:6040 .. 2000:6368");
        out.println();
        out.println("Notes");
        out.println("- This is the post-planet, pre-IPBM validator branch inside 2000:5EE4.");
        out.println("- It opens the 0x36-byte stream at 0x3178, so this branch is the FLEETS.DAT path.");
        out.println("- Local buffer anchored at [BP+0xFF3E] is the active fleet record scratch.");
        out.println();
    }

    private void dumpRange(PrintWriter out, String start, String end, String label) {
        out.println(label + " (" + start + " .. " + end + ")");
        Address curr = toAddr(start);
        Address stop = toAddr(end);
        while (curr.compareTo(stop) <= 0) {
            Instruction inst = getInstructionAt(curr);
            if (inst == null) {
                disassemble(curr);
                inst = getInstructionContaining(curr);
            }
            if (inst == null) {
                curr = curr.add(1);
                continue;
            }
            out.printf("- %s  %s", inst.getAddress(), inst);
            Function fn = getFunctionContaining(inst.getAddress());
            if (fn != null && inst.getAddress().equals(fn.getEntryPoint())) {
                out.printf("    [function %s]", fn.getName());
            }
            for (Reference ref : inst.getReferencesFrom()) {
                if (ref.getToAddress() != null) {
                    out.printf("    [ref %s]", ref.getToAddress());
                }
            }
            for (int i = 0; i < inst.getNumOperands(); i++) {
                for (Object obj : inst.getOpObjects(i)) {
                    if (obj instanceof Scalar scalar) {
                        out.printf("    [scalar 0x%x]", scalar.getUnsignedValue());
                    } else if (obj instanceof Address address) {
                        out.printf("    [addr %s]", address);
                    }
                }
            }
            out.println();
            curr = inst.getAddress().add(inst.getLength());
        }
    }
}
