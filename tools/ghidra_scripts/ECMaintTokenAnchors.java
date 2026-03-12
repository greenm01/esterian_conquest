//@category EsterianConquest

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;
import ghidra.program.model.symbol.SourceType;

public class ECMaintTokenAnchors extends GhidraScript {

	private static final String[][] ANCHORS = new String[][] {
		{"2000:841b", "ecmaint_main_tok_guard_strings"},
		{"2000:6fc6", "ecmaint_conquest_tok_cleanup_strings"},
		{"2000:9680", "ecmaint_generic_token_wait_delete_strings"},
		{"2000:96c4", "ecmaint_check_named_token_candidate"},
		{"2000:9887", "ecmaint_delete_named_token_candidate"},
		{"2000:9c91", "ecmaint_move_tok_check_wrapper_candidate"},
		{"2000:9cb0", "ecmaint_move_tok_delete_wrapper_candidate"},
		{"2000:9b13", "ecmaint_token_wait_timeout_helper"},
		{"2000:945b", "ecmaint_emit_timestamp_message_helper"},
		{"2000:9d48", "ecmaint_move_tok_recovery_candidate"},
		{"2000:9e1e", "ecmaint_wait_for_named_token_candidate"},
		{"3000:39dc", "ecmaint_time_query_helper_candidate"},
		{"4000:0f70", "ecmaint_conquest_tok_string_a"},
		{"4000:1102", "ecmaint_conquest_tok_string_b"}
	};

	@Override
	protected void run() throws Exception {
		String[] args = getScriptArgs();
		File outputDir = args.length >= 1 ? new File(args[0]) : new File("artifacts/ghidra/ecmaint-live");
		if (!outputDir.exists() && !outputDir.mkdirs()) {
			throw new IllegalStateException("failed to create output directory: " + outputDir);
		}

		for (String[] pair : ANCHORS) {
			createLabel(toAddr(pair[0]), pair[1], true, SourceType.USER_DEFINED);
		}
		renameFunction("2000:96c4", "ecmaint_check_named_token_candidate");
		renameFunction("2000:9887", "ecmaint_delete_named_token_candidate");
		renameFunction("2000:9c91", "ecmaint_move_tok_check_wrapper_candidate");
		renameFunction("2000:9cb0", "ecmaint_move_tok_delete_wrapper_candidate");
		renameFunction("2000:945b", "ecmaint_emit_timestamp_message_helper");
		renameFunction("2000:9b13", "ecmaint_token_wait_timeout_helper");
		renameFunction("2000:9d48", "ecmaint_move_tok_recovery_candidate");
		renameFunction("2000:9e1e", "ecmaint_wait_for_named_token_candidate");

		File report = new File(outputDir, "token-anchors.txt");
		try (PrintWriter out = new PrintWriter(new FileWriter(report))) {
			out.printf("Program: %s%n%n", currentProgram.getName());
			for (String[] pair : ANCHORS) {
				writeAnchor(out, pair[0], pair[1]);
			}
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
				println("Could not create function at " + address + "; keeping label only for " + name);
				return;
			}
			return;
		}
		function.setName(name, SourceType.USER_DEFINED);
	}

	private void writeAnchor(PrintWriter out, String addressText, String label) throws Exception {
		Address address = toAddr(addressText);
		out.printf("%s %s%n", address, label);
		out.printf("- strings:%n");
		for (String text : readAsciiStrings(address, 8, 120)) {
			out.printf("  - %s%n", text);
		}

		Function function = getFunctionContaining(address);
		out.printf("- containing function: %s%n", function == null ? "<none>" : function.getEntryPoint() + " " + function.getName());

		out.printf("- references to anchor:%n");
		ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(address);
		int refCount = 0;
		while (refs.hasNext() && !monitor.isCancelled()) {
			Reference ref = refs.next();
			Function caller = getFunctionContaining(ref.getFromAddress());
			String callerText = caller == null ? "<no-function>" : caller.getEntryPoint() + " " + caller.getName();
			out.printf("  - %s (%s)%n", ref.getFromAddress(), callerText);
			refCount++;
		}
		if (refCount == 0) {
			out.printf("  - <none>%n");
		}

		out.printf("- nearby instructions:%n");
		Instruction instruction = getInstructionBefore(address);
		int skipped = 0;
		while (instruction != null && skipped < 6) {
			instruction = instruction.getPrevious();
			skipped++;
		}
		if (instruction == null) {
			instruction = getInstructionAt(address);
		}
		int count = 0;
		while (instruction != null && count < 18 && !monitor.isCancelled()) {
			out.printf("  - %s  %s%n", instruction.getAddress(), instruction);
			instruction = instruction.getNext();
			count++;
		}
		out.println();
	}

	private List<String> readAsciiStrings(Address start, int maxStrings, int maxLength) throws Exception {
		List<String> strings = new ArrayList<>();
		Memory memory = currentProgram.getMemory();
		Address cursor = start;
		int attempts = 0;
		while (strings.size() < maxStrings && attempts < maxStrings * 4 && !monitor.isCancelled()) {
			String text = readAsciiString(memory, cursor, maxLength);
			if (text.length() >= 4) {
				strings.add(text);
				cursor = cursor.add(text.length() + 1L);
			}
			else {
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
