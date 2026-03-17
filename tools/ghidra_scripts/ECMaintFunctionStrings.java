//@category EsterianConquest

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;
import java.util.Arrays;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Set;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.scalar.Scalar;

public class ECMaintFunctionStrings extends GhidraScript {

    private static final List<String> TARGETS = Arrays.asList(
        "2000:861d",
        "2000:1da6",
        "2000:0c06",
        "2000:2db3",
        "2000:56be",
        "2000:7659",
        "3000:1abc",
        "3000:1e88"
    );

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        File outputDir = args.length >= 1 ? new File(args[0]) : new File("artifacts/ghidra/ecmaint-live");
        if (!outputDir.exists() && !outputDir.mkdirs()) {
            throw new IllegalStateException("failed to create output directory: " + outputDir);
        }

        File report = new File(outputDir, "turn-cycle-function-strings.txt");
        try (PrintWriter out = new PrintWriter(new FileWriter(report))) {
            out.printf("Program: %s%n%n", currentProgram.getName());
            for (String target : TARGETS) {
                writeTargetStrings(out, target);
            }
        }
        println("Wrote " + report.getAbsolutePath());
    }

    private void writeTargetStrings(PrintWriter out, String addressText) {
        Address address = toAddr(addressText);
        Function function = getFunctionContaining(address);
        out.printf("Target %s%n", address);
        out.printf("- containing function: %s%n",
            function == null ? "<none>" : function.getEntryPoint() + " " + function.getName());
        if (function == null) {
            out.println("- strings:");
            out.println("  - <no function>");
            out.println();
            return;
        }

        Address start = function.getEntryPoint();
        Address end = function.getBody().getMaxAddress();
        Set<String> lines = new LinkedHashSet<>();

        Instruction ins = getInstructionAt(start);
        while (ins != null && ins.getAddress().compareTo(end) <= 0 && !monitor.isCancelled()) {
            if ("MOV".equals(ins.getMnemonicString()) && ins.getNumOperands() >= 2) {
                Object[] op0 = ins.getOpObjects(0);
                Object[] op1 = ins.getOpObjects(1);
                if (op0.length == 1 && op1.length == 1 && op0[0] instanceof ghidra.program.model.lang.Register &&
                    op1[0] instanceof Scalar && "DI".equals(((ghidra.program.model.lang.Register) op0[0]).getName())) {
                    long offset = ((Scalar) op1[0]).getUnsignedValue();
                    Address candidate = toAddr(String.format("%s:%04x",
                        start.toString().substring(0, 4), offset));
                    String text = readAscii(candidate);
                    if (text != null && !text.isEmpty()) {
                        lines.add(String.format("  - %s -> %s", candidate, text));
                    }
                }
            }
            ins = ins.getNext();
        }

        out.println("- strings:");
        if (lines.isEmpty()) {
            out.println("  - <none>");
        } else {
            for (String line : lines) {
                out.println(line);
            }
        }
        out.println();
    }

    private String readAscii(Address address) {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < 120; i++) {
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
                if (sb.length() == 0) {
                    return null;
                }
                break;
            }
            sb.append((char) value);
        }
        return sb.toString();
    }
}
