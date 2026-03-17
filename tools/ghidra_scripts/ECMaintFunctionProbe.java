//@category EsterianConquest

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;
import java.util.LinkedHashSet;
import java.util.Set;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.lang.Register;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.scalar.Scalar;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

public class ECMaintFunctionProbe extends GhidraScript {

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length == 0) {
            throw new IllegalArgumentException("usage: ECMaintFunctionProbe.java <address> [output_dir]");
        }
        String target = args[0];
        File outputDir = args.length >= 2 ? new File(args[1]) : new File("artifacts/ghidra/ecmaint-live");
        if (!outputDir.exists() && !outputDir.mkdirs()) {
            throw new IllegalStateException("failed to create output directory: " + outputDir);
        }

        File report = new File(outputDir, "probe-" + target.replace(':', '_') + ".txt");
        try (PrintWriter out = new PrintWriter(new FileWriter(report))) {
            writeProbe(out, target);
        }
        println("Wrote " + report.getAbsolutePath());
    }

    private void writeProbe(PrintWriter out, String addressText) throws Exception {
        Address address = toAddr(addressText);
        Function function = getFunctionContaining(address);
        out.printf("Target %s%n", address);
        out.printf("- containing function: %s%n",
            function == null ? "<none>" : function.getEntryPoint() + " " + function.getName());
        out.println("- incoming refs:");
        ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(address);
        int refCount = 0;
        while (refs.hasNext() && !monitor.isCancelled()) {
            Reference ref = refs.next();
            out.printf("  - %s (%s)%n", ref.getFromAddress(), ref.getReferenceType());
            refCount++;
        }
        if (refCount == 0) {
            out.println("  - <none>");
        }

        out.println("- local window:");
        writeWindowAround(out, address, 40, 160);

        if (function == null) {
            return;
        }

        out.println("- calls:");
        Set<String> calls = new LinkedHashSet<>();
        out.println("- constants:");
        Set<String> constants = new LinkedHashSet<>();

        Instruction ins = getInstructionAt(function.getEntryPoint());
        Address end = function.getBody().getMaxAddress();
        while (ins != null && ins.getAddress().compareTo(end) <= 0 && !monitor.isCancelled()) {
            if (ins.getMnemonicString().startsWith("CALL")) {
                calls.add("  - " + ins.getAddress() + "  " + ins);
            }
            collectConstants(function, ins, constants);
            ins = ins.getNext();
        }

        if (calls.isEmpty()) {
            out.println("  - <none>");
        } else {
            for (String line : calls) {
                out.println(line);
            }
        }
        out.println("- constants:");
        if (constants.isEmpty()) {
            out.println("  - <none>");
        } else {
            for (String line : constants) {
                out.println(line);
            }
        }
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
            Address candidate = toAddr(String.format("%s:%04x",
                function.getEntryPoint().toString().substring(0, 4), value));
            String text = readAscii(candidate);
            if ("MOV".equals(ins.getMnemonicString())) {
                Object[] op0 = ins.getOpObjects(0);
                if (op0.length == 1 && op0[0] instanceof Register &&
                    "DI".equals(((Register) op0[0]).getName())) {
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

    private void writeWindowAround(PrintWriter out, Address center, int backCount, int forwardCount) {
        Instruction ins = getInstructionContaining(center);
        if (ins == null) {
            ins = getInstructionAt(center);
        }
        if (ins == null) {
            out.printf("  %s <no instruction>%n", center);
            return;
        }
        for (int i = 0; i < backCount; i++) {
            Instruction prev = ins.getPrevious();
            if (prev == null) {
                break;
            }
            ins = prev;
        }
        int emitted = 0;
        while (ins != null && emitted < backCount + forwardCount && !monitor.isCancelled()) {
            out.printf("  %s  %s%n", ins.getAddress(), ins);
            ins = ins.getNext();
            emitted++;
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
