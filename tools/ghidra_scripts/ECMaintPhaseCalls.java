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
import ghidra.program.model.lang.Register;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.scalar.Scalar;

public class ECMaintPhaseCalls extends GhidraScript {

    private static final List<String> TARGETS = Arrays.asList(
        "2000:1da6",
        "2000:0c06",
        "2000:2db3",
        "2000:56be"
    );

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        File outputDir = args.length >= 1 ? new File(args[0]) : new File("artifacts/ghidra/ecmaint-live");
        if (!outputDir.exists() && !outputDir.mkdirs()) {
            throw new IllegalStateException("failed to create output directory: " + outputDir);
        }

        File report = new File(outputDir, "phase-calls.txt");
        try (PrintWriter out = new PrintWriter(new FileWriter(report))) {
            out.printf("Program: %s%n%n", currentProgram.getName());
            for (String target : TARGETS) {
                writeFunction(out, target);
            }
        }
        println("Wrote " + report.getAbsolutePath());
    }

    private void writeFunction(PrintWriter out, String addressText) throws Exception {
        Address address = toAddr(addressText);
        Function function = getFunctionContaining(address);
        out.printf("Target %s%n", address);
        out.printf("- containing function: %s%n",
            function == null ? "<none>" : function.getEntryPoint() + " " + function.getName());
        if (function == null) {
            out.println("- <no function>");
            out.println();
            return;
        }

        Set<String> calls = new LinkedHashSet<>();
        Set<String> constants = new LinkedHashSet<>();
        Instruction ins = getInstructionAt(function.getEntryPoint());
        Address end = function.getBody().getMaxAddress();
        while (ins != null && ins.getAddress().compareTo(end) <= 0 && !monitor.isCancelled()) {
            String mnemonic = ins.getMnemonicString();
            if (mnemonic.startsWith("CALL")) {
                calls.add(String.format("  - %s  %s", ins.getAddress(), ins));
            }
            collectConstants(function, ins, constants);
            ins = ins.getNext();
        }

        out.println("- calls:");
        if (calls.isEmpty()) {
            out.println("  - <none>");
        } else {
            for (String line : calls) {
                out.println(line);
            }
        }

        out.println("- candidate constants/strings:");
        if (constants.isEmpty()) {
            out.println("  - <none>");
        } else {
            for (String line : constants) {
                out.println(line);
            }
        }
        out.println();
    }

    private void collectConstants(Function function, Instruction ins, Set<String> constants) throws Exception {
        for (int operand = 0; operand < ins.getNumOperands(); operand++) {
            Object[] objects = ins.getOpObjects(operand);
            if (objects.length != 1 || !(objects[0] instanceof Scalar)) {
                continue;
            }
            long value = ((Scalar) objects[0]).getUnsignedValue();
            if (value > 0xffff) {
                continue;
            }
            if ("MOV".equals(ins.getMnemonicString())) {
                Object[] op0 = ins.getOpObjects(0);
                if (op0.length == 1 && op0[0] instanceof Register &&
                    "DI".equals(((Register) op0[0]).getName())) {
                    Address candidate = toAddr(String.format("%s:%04x",
                        function.getEntryPoint().toString().substring(0, 4), value));
                    String text = readAscii(candidate);
                    if (text != null && !text.isEmpty()) {
                        constants.add(String.format("  - %s via %s -> %s", candidate, ins.getAddress(), text));
                    } else {
                        constants.add(String.format("  - scalar 0x%04x via %s", value, ins.getAddress()));
                    }
                }
            } else if (ins.getMnemonicString().startsWith("PUSH")) {
                constants.add(String.format("  - scalar 0x%04x via %s", value, ins.getAddress()));
            }
        }
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
