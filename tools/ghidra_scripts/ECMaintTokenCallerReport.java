//@category EsterianConquest

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

public class ECMaintTokenCallerReport extends GhidraScript {

	private static final String[][] TARGETS = new String[][] {
		{"2000:9d48", "ecmaint_move_tok_recovery_candidate"},
		{"2000:9e1e", "ecmaint_wait_for_named_token_candidate"}
	};

	private static final String[][] RANGES = new String[][] {
		{"2000:7200", "2000:7360", "dynamic 96c4 caller vicinity"},
		{"2000:9e43", "2000:ac40", "post-wait caller gap"},
		{"2000:9c88", "2000:9e42", "known local token helper block"}
	};

	@Override
	protected void run() throws Exception {
		String[] args = getScriptArgs();
		File outputDir = args.length >= 1 ? new File(args[0]) : new File("artifacts/ghidra/ecmaint-live");
		if (!outputDir.exists() && !outputDir.mkdirs()) {
			throw new IllegalStateException("failed to create output directory: " + outputDir);
		}

		File report = new File(outputDir, "token-callers.txt");
		try (PrintWriter out = new PrintWriter(new FileWriter(report))) {
			out.printf("Program: %s%n%n", currentProgram.getName());
			for (String[] target : TARGETS) {
				writeReferences(out, target[0], target[1]);
			}
			writeStackDerivedLead(out);
			for (String[] range : RANGES) {
				writeRangeScan(out, range[0], range[1], range[2]);
			}
		}
		println("Wrote " + report.getAbsolutePath());
	}

	private void writeStackDerivedLead(PrintWriter out) throws Exception {
		out.println("Dynamic lead from 96c4 stack capture");
		Address derivedArg = toAddr("2000:7232");
		Address derivedReturn = toAddr("2000:7338");
		out.printf("- incoming far-pointer candidate: %s%n", derivedArg);
		out.printf("- caller/return-site vicinity: %s%n", derivedReturn);
		writeNearby(out, derivedArg);
		writeNearby(out, derivedReturn);
		out.println();
	}

	private void writeReferences(PrintWriter out, String addressText, String label) throws Exception {
		Address address = toAddr(addressText);
		out.printf("%s %s%n", address, label);
		ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(address);
		int refCount = 0;
		while (refs.hasNext() && !monitor.isCancelled()) {
			Reference ref = refs.next();
			Instruction instruction = getInstructionContaining(ref.getFromAddress());
			Function caller = getFunctionContaining(ref.getFromAddress());
			out.printf("- ref from %s", ref.getFromAddress());
			if (instruction != null) {
				out.printf("  %s", instruction);
			}
			if (caller != null) {
				out.printf("  [function %s %s]", caller.getEntryPoint(), caller.getName());
			}
			out.println();
			refCount++;
		}
		if (refCount == 0) {
			out.println("- <none>");
		}
		out.println();
	}

	private void writeRangeScan(PrintWriter out, String startText, String endText, String label) throws Exception {
		Address start = toAddr(startText);
		Address end = toAddr(endText);
		out.printf("%s (%s .. %s)%n", label, start, end);

		Instruction instruction = ensureInstruction(start);
		int directRefCount = 0;
		int prologueCount = 0;
		while (instruction != null && instruction.getAddress().compareTo(end) <= 0 && !monitor.isCancelled()) {
			Reference[] refs = instruction.getReferencesFrom();
			boolean wrote = false;
			for (Reference ref : refs) {
				Address to = ref.getToAddress();
				if (to == null) {
					continue;
				}
				if (to.equals(toAddr("2000:9d48")) || to.equals(toAddr("2000:9e1e"))) {
					Function caller = getFunctionContaining(instruction.getAddress());
					out.printf("- direct ref %s  %s", instruction.getAddress(), instruction);
					if (caller != null) {
						out.printf("  [function %s %s]", caller.getEntryPoint(), caller.getName());
					}
					out.printf("  [to %s]%n", to);
					directRefCount++;
					wrote = true;
				}
			}

			if (!wrote && looksLikePrologue(instruction)) {
				Function function = getFunctionAt(instruction.getAddress());
				out.printf("- prologue candidate %s", instruction.getAddress());
				if (function != null) {
					out.printf("  [function %s %s]", function.getEntryPoint(), function.getName());
				}
				out.println();
				prologueCount++;
			}

			instruction = nextInstruction(instruction, end);
		}

		if (directRefCount == 0) {
			out.println("- no direct refs to 2000:9d48 or 2000:9e1e found in this range");
		}
		out.printf("- prologue candidates seen: %d%n", prologueCount);
		out.println();
	}

	private void writeNearby(PrintWriter out, Address center) throws Exception {
		out.printf("- nearby at %s%n", center);
		Address start = center.subtract(0x20);
		Address end = center.add(0x20);
		Instruction instruction = ensureInstruction(start);
		int count = 0;
		while (instruction != null && instruction.getAddress().compareTo(end) <= 0 && count < 24 && !monitor.isCancelled()) {
			out.printf("  - %s  %s%n", instruction.getAddress(), instruction);
			instruction = nextInstruction(instruction, end);
			count++;
		}
	}

	private Instruction ensureInstruction(Address address) throws Exception {
		Instruction instruction = getInstructionAt(address);
		if (instruction != null) {
			return instruction;
		}
		disassemble(address);
		instruction = getInstructionContaining(address);
		if (instruction != null) {
			return instruction;
		}
		Address cursor = address;
		for (int i = 0; i < 32 && instruction == null; i++) {
			cursor = cursor.add(1);
			disassemble(cursor);
			instruction = getInstructionContaining(cursor);
		}
		return instruction;
	}

	private Instruction nextInstruction(Instruction instruction, Address end) throws Exception {
		Instruction next = instruction.getNext();
		if (next != null) {
			return next;
		}
		Address cursor = instruction.getMaxAddress().add(1);
		while (cursor.compareTo(end) <= 0 && !monitor.isCancelled()) {
			disassemble(cursor);
			next = getInstructionContaining(cursor);
			if (next != null && next.getAddress().compareTo(instruction.getAddress()) > 0) {
				return next;
			}
			cursor = cursor.add(1);
		}
		return null;
	}

	private boolean looksLikePrologue(Instruction instruction) {
		if (instruction == null || !"PUSH".equals(instruction.getMnemonicString())) {
			return false;
		}
		Object[] opObjects = instruction.getOpObjects(0);
		if (opObjects.length != 1 || !"BP".equals(opObjects[0].toString())) {
			return false;
		}
		Instruction next = instruction.getNext();
		return next != null
			&& "MOV".equals(next.getMnemonicString())
			&& next.toString().startsWith("MOV BP,SP");
	}
}
