//@category EsterianConquest

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;
import java.util.Arrays;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.scalar.Scalar;

public class ECMaintStringXrefs extends GhidraScript {

    private static final Map<Long, String> TARGETS = new LinkedHashMap<>();
    static {
        TARGETS.put(0x841bL, "startup_main_tok_cluster");
        TARGETS.put(0x84e9L, "startup_create_main_work_file");
        TARGETS.put(0x8504L, "startup_done_after_main_work_file");
        TARGETS.put(0x853dL, "startup_merge_joint_fleets");
        TARGETS.put(0x855aL, "startup_starbase_merge_report");
        TARGETS.put(0x7c44L, "pre_restore_status_string");
        TARGETS.put(0x7ca1L, "restore_failure_status_string");
        TARGETS.put(0x7cd8L, "post_pipeline_status_string");
        TARGETS.put(0x7cf3L, "late_sort_status_string");
        TARGETS.put(0x7cf8L, "late_sort_suffix_string");
    }

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        File outputDir = args.length >= 1 ? new File(args[0]) : new File("artifacts/ghidra/ecmaint-live");
        if (!outputDir.exists() && !outputDir.mkdirs()) {
            throw new IllegalStateException("failed to create output directory: " + outputDir);
        }

        File report = new File(outputDir, "string-xrefs.txt");
        try (PrintWriter out = new PrintWriter(new FileWriter(report))) {
            out.printf("Program: %s%n%n", currentProgram.getName());
            for (Map.Entry<Long, String> entry : TARGETS.entrySet()) {
                writeMatches(out, entry.getKey(), entry.getValue());
            }
        }
        println("Wrote " + report.getAbsolutePath());
    }

    private void writeMatches(PrintWriter out, long value, String label) throws Exception {
        Address target = toAddr(String.format("2000:%04x", value));
        out.printf("Target 2000:%04x %s%n", value, label);
        out.printf("- decoded text: %s%n", readAscii(target));

        int matches = 0;
        InstructionIterator iter = currentProgram.getListing().getInstructions(true);
        while (iter.hasNext() && !monitor.isCancelled()) {
            Instruction ins = iter.next();
            if (!containsScalar(ins, value)) {
                continue;
            }
            matches++;
            out.printf("  - %s  %s%n", ins.getAddress(), ins);
            dumpWindow(out, ins, 6, 14);
        }
        if (matches == 0) {
            out.println("  - <no scalar hits>");
        }
        out.println();
    }

    private boolean containsScalar(Instruction ins, long value) {
        for (int operand = 0; operand < ins.getNumOperands(); operand++) {
            Object[] objects = ins.getOpObjects(operand);
            for (Object object : objects) {
                if (object instanceof Scalar && ((Scalar) object).getUnsignedValue() == value) {
                    return true;
                }
            }
        }
        return false;
    }

    private void dumpWindow(PrintWriter out, Instruction center, int back, int forward) {
        Instruction start = center;
        for (int i = 0; i < back; i++) {
            Instruction prev = start.getPrevious();
            if (prev == null) {
                break;
            }
            start = prev;
        }
        Instruction ins = start;
        int emitted = 0;
        int total = back + forward + 1;
        while (ins != null && emitted < total && !monitor.isCancelled()) {
            out.printf("      %s  %s%n", ins.getAddress(), ins);
            ins = ins.getNext();
            emitted++;
        }
    }

    private String readAscii(Address address) {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < 160; i++) {
            byte b;
            try {
                b = getByte(address.add(i));
            } catch (Exception e) {
                break;
            }
            int value = b & 0xff;
            if (value == 0) {
                break;
            }
            if (value < 32 || value > 126) {
                break;
            }
            sb.append((char) value);
        }
        return sb.toString();
    }
}
