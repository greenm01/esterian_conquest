//@category EsterianConquest

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.scalar.Scalar;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;
import ghidra.program.model.symbol.SourceType;

public class ECMaintTimingFlow extends GhidraScript {

    private static final String TIMESTAMP_HELPER = "2000:945b";
    private static final String SCHEDULE_GATE = "2000:6fc6";
    private static final String TIME_QUERY_HELPER = "3000:39dc";
    private static final String RANKING_STRING = "3000:189c";

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        File outputDir = args.length >= 1 ? new File(args[0]) : new File("artifacts/ghidra/ecmaint-live");
        if (!outputDir.exists() && !outputDir.mkdirs()) {
            throw new IllegalStateException("failed to create output directory: " + outputDir);
        }

        renameFunction(TIMESTAMP_HELPER, "ecmaint_emit_timestamp_message_helper");
        renameFunction(TIME_QUERY_HELPER, "ecmaint_time_query_helper_candidate");

        File report = new File(outputDir, "timing-flow.txt");
        try (PrintWriter out = new PrintWriter(new FileWriter(report))) {
            out.printf("Program: %s%n%n", currentProgram.getName());
            writeTimestampSection(out);
            writeAnchorSection(
                out,
                "Maintenance schedule gate",
                SCHEDULE_GATE,
                Arrays.asList(
                    "schedule string cluster containing the 'Today is ... maintenance is not scheduled to run' text",
                    "used to anchor the date-gating path that consults CONQUEST scheduling"
                )
            );
            writeAnchorSection(
                out,
                "Time query helper candidate",
                TIME_QUERY_HELPER,
                Arrays.asList(
                    "candidate low-level date/time query helper already labeled by prior token-anchor work",
                    "expected to feed the maintenance schedule gate rather than report emission directly"
                )
            );
            writeAnchorSection(
                out,
                "Ranking-generation string cluster",
                RANKING_STRING,
                Arrays.asList(
                    "string cluster containing 'Enabling player-ranking text file generation...'",
                    "used as a static anchor for the rankings-output path that now includes stardates"
                )
            );
        }
        println("Wrote " + report.getAbsolutePath());
    }

    private void renameFunction(String addressText, String name) throws Exception {
        Address address = toAddr(addressText);
        Function function = getFunctionAt(address);
        if (function == null) {
            disassemble(address);
            function = createFunction(address, name);
            if (function == null) {
                println("Could not create function at " + address + " for " + name);
                return;
            }
        }
        function.setName(name, SourceType.USER_DEFINED);
    }

    private void writeTimestampSection(PrintWriter out) throws Exception {
        Address helper = toAddr(TIMESTAMP_HELPER);
        out.println("Timestamp helper");
        out.printf("- address: %s%n", helper);
        Function function = getFunctionAt(helper);
        out.printf("- function: %s%n", function == null ? "<none>" : function.getEntryPoint() + " " + function.getName());
        out.printf("- anchor strings near helper:%n");
        for (String text : readAsciiStrings(helper, 6, 120)) {
            out.printf("  - %s%n", text);
        }
        out.printf("- direct references:%n");
        List<Reference> refs = referencesTo(helper);
        if (refs.isEmpty()) {
            out.println("  - <none>");
        }
        for (Reference ref : refs) {
            Function caller = getFunctionContaining(ref.getFromAddress());
            out.printf(
                "  - %s (%s, %s)%n",
                ref.getFromAddress(),
                ref.getReferenceType(),
                caller == null ? "<no-function>" : caller.getEntryPoint() + " " + caller.getName()
            );
        }
        out.printf("- caller contexts:%n");
        if (refs.isEmpty()) {
            out.println("  - <none>");
        }
        for (Reference ref : refs) {
            writeInstructionWindow(out, ref.getFromAddress(), 5, 12);
            writeStackAndGlobalTouches(out, ref.getFromAddress(), 12);
        }
        out.printf("- helper body:%n");
        writeInstructionWindow(out, helper, 0, 28);
        out.println();
    }

    private void writeAnchorSection(PrintWriter out, String title, String addressText, List<String> notes) throws Exception {
        Address address = toAddr(addressText);
        out.println(title);
        out.printf("- address: %s%n", address);
        for (String note : notes) {
            out.printf("- note: %s%n", note);
        }
        Function function = getFunctionContaining(address);
        out.printf("- containing function: %s%n",
            function == null ? "<none>" : function.getEntryPoint() + " " + function.getName());
        out.printf("- nearby strings:%n");
        for (String text : readAsciiStrings(address, 6, 120)) {
            out.printf("  - %s%n", text);
        }
        out.printf("- direct references:%n");
        List<Reference> refs = referencesTo(address);
        if (refs.isEmpty()) {
            out.println("  - <none>");
        }
        for (Reference ref : refs) {
            Function caller = getFunctionContaining(ref.getFromAddress());
            out.printf(
                "  - %s (%s, %s)%n",
                ref.getFromAddress(),
                ref.getReferenceType(),
                caller == null ? "<no-function>" : caller.getEntryPoint() + " " + caller.getName()
            );
        }
        out.printf("- local disassembly window:%n");
        writeInstructionWindow(out, address, 6, 22);
        out.println();
    }

    private void writeInstructionWindow(PrintWriter out, Address center, int beforeCount, int totalCount) {
        Instruction instruction = getInstructionContaining(center);
        if (instruction == null) {
            instruction = getInstructionAt(center);
        }
        if (instruction == null) {
            out.printf("  - %s <no instruction>%n", center);
            return;
        }
        for (int i = 0; i < beforeCount; i++) {
            Instruction previous = instruction.getPrevious();
            if (previous == null) {
                break;
            }
            instruction = previous;
        }
        int emitted = 0;
        while (instruction != null && emitted < totalCount && !monitor.isCancelled()) {
            out.printf("  - %s  %s%n", instruction.getAddress(), instruction);
            instruction = instruction.getNext();
            emitted++;
        }
    }

    private void writeStackAndGlobalTouches(PrintWriter out, Address center, int instructionCount) {
        Instruction instruction = getInstructionContaining(center);
        if (instruction == null) {
            instruction = getInstructionAt(center);
        }
        if (instruction == null) {
            return;
        }
        out.println("  - nearby stack/global operands:");
        int emitted = 0;
        while (instruction != null && emitted < instructionCount && !monitor.isCancelled()) {
            String operands = describeOperands(instruction);
            if (!operands.isEmpty()) {
                out.printf("    - %s -> %s%n", instruction.getAddress(), operands);
            }
            instruction = instruction.getNext();
            emitted++;
        }
    }

    private String describeOperands(Instruction instruction) {
        List<String> parts = new ArrayList<>();
        for (int i = 0; i < instruction.getNumOperands(); i++) {
            Object[] objects = instruction.getOpObjects(i);
            for (Object object : objects) {
                if (object instanceof Scalar) {
                    long value = ((Scalar) object).getSignedValue();
                    if (Math.abs(value) >= 0x1000) {
                        parts.add(String.format("scalar[%d]=0x%X", i, value));
                    }
                } else {
                    String text = object.toString();
                    if (text.contains("BP") || text.contains("SP") || text.contains("SS:[") || text.contains("DS:[")
                        || text.matches(".*\\[[0-9a-fA-Fx:]+\\].*")) {
                        parts.add("op" + i + "=" + text);
                    }
                }
            }
        }
        return String.join(", ", parts);
    }

    private List<Reference> referencesTo(Address address) {
        List<Reference> refs = new ArrayList<>();
        ReferenceIterator it = currentProgram.getReferenceManager().getReferencesTo(address);
        while (it.hasNext() && !monitor.isCancelled()) {
            refs.add(it.next());
        }
        return refs;
    }

    private List<String> readAsciiStrings(Address start, int maxStrings, int maxLength) throws Exception {
        List<String> strings = new ArrayList<>();
        Memory memory = currentProgram.getMemory();
        Address cursor = start;
        int attempts = 0;
        while (strings.size() < maxStrings && attempts < maxStrings * 6 && !monitor.isCancelled()) {
            String text = readAsciiString(memory, cursor, maxLength);
            if (text.length() >= 4) {
                strings.add(text);
                cursor = cursor.add(text.length() + 1L);
            } else {
                cursor = cursor.add(1L);
            }
            attempts++;
        }
        return strings;
    }

    private String readAsciiString(Memory memory, Address start, int maxLength) throws Exception {
        byte[] buffer = new byte[maxLength];
        int size = 0;
        for (int i = 0; i < maxLength; i++) {
            byte value = memory.getByte(start.add(i));
            if (value == 0) {
                break;
            }
            if (value < 0x20 || value > 0x7e) {
                break;
            }
            buffer[size++] = value;
        }
        return new String(buffer, 0, size, StandardCharsets.US_ASCII);
    }
}
