//@category EsterianConquest

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;
import java.util.ArrayList;
import java.util.List;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;
import ghidra.program.model.symbol.SourceType;

public class ECMaintNameIntegrityAnchors extends GhidraScript {

	private static final String[][] LABELS = new String[][] {
		{"2000:6d98", "ecmaint_integrity_string_cluster"},
		{"2000:6d9b", "ecmaint_integrity_restore_entry"},
		{"2000:5ee4", "ecmaint_validate_primary_state"},
		{"2000:6f7c", "ecmaint_integrity_recursive_backup_call"},
		{"2000:841b", "ecmaint_main_tok_guard_strings"},
		{"2000:96f8", "ecmaint_token_wait_delete_strings"}
	};

	@Override
	protected void run() throws Exception {
		String[] args = getScriptArgs();
		File outputDir = args.length >= 1 ? new File(args[0]) : new File("artifacts/ghidra/ecmaint-live");
		if (!outputDir.exists() && !outputDir.mkdirs()) {
			throw new IllegalStateException("failed to create output directory: " + outputDir);
		}

		List<String> notes = new ArrayList<>();
		for (String[] pair : LABELS) {
			Address address = toAddr(pair[0]);
			createLabel(address, pair[1], true, SourceType.USER_DEFINED);
			notes.add(String.format("%s %s", address, pair[1]));
		}

		Address entry = toAddr("2000:6d9b");
		Function entryFunction = getFunctionAt(entry);
		if (entryFunction == null) {
			disassemble(entry);
			entryFunction = createFunction(entry, "ecmaint_integrity_restore_entry");
			if (entryFunction == null) {
				throw new IllegalStateException("failed to create function at " + entry);
			}
		}
		else {
			entryFunction.setName("ecmaint_integrity_restore_entry", SourceType.USER_DEFINED);
		}

		renameFunction("2000:5ee4", "ecmaint_validate_primary_state");

		writeReport(new File(outputDir, "integrity-anchors.txt"), notes, entry);
	}

	private void renameFunction(String addressText, String name) throws Exception {
		Address address = toAddr(addressText);
		Function function = getFunctionAt(address);
		if (function == null) {
			disassemble(address);
			function = createFunction(address, name);
			if (function == null) {
				throw new IllegalStateException("failed to create function at " + address);
			}
			return;
		}
		function.setName(name, SourceType.USER_DEFINED);
	}

	private void writeReport(File outputFile, List<String> notes, Address entry) throws Exception {
		try (PrintWriter out = new PrintWriter(new FileWriter(outputFile))) {
			out.printf("Program: %s%n%n", currentProgram.getName());
			out.println("Named anchors:");
			for (String note : notes) {
				out.printf("- %s%n", note);
			}
			out.println();

			out.printf("Function at %s%n", entry);
			Function function = getFunctionAt(entry);
			out.printf("- name: %s%n", function == null ? "<missing>" : function.getName());

			out.println("- incoming references:");
			ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(entry);
			int refCount = 0;
			while (refs.hasNext() && !monitor.isCancelled()) {
				Reference ref = refs.next();
				Function caller = getFunctionContaining(ref.getFromAddress());
				String callerName = caller == null ? "<no-function>" : caller.getName();
				out.printf("  - %s (%s)%n", ref.getFromAddress(), callerName);
				refCount++;
			}
			if (refCount == 0) {
				out.println("  - <none>");
			}

			out.println("- first instructions:");
			Instruction instruction = getInstructionAt(entry);
			int count = 0;
			while (instruction != null && count < 16 && !monitor.isCancelled()) {
				out.printf("  - %s  %s%n", instruction.getAddress(), instruction);
				instruction = instruction.getNext();
				count++;
			}
		}
		println("Wrote " + outputFile.getAbsolutePath());
	}
}
