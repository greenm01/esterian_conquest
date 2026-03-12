//@category EsterianConquest

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.lang.Register;
import ghidra.program.model.listing.Data;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.scalar.Scalar;
import ghidra.program.model.symbol.Reference;

public class ECMaintTokenReachabilityReport extends GhidraScript {

	private static final String[][] CODE_RANGES = new String[][] {
		{"2000:945b", "2000:967c", "counted-string / token message helper range"},
		{"2000:9c88", "2000:9cc2", "move-token tiny helper prelude"},
		{"2000:96c4", "2000:9837", "move-token helper callee A"},
		{"2000:9887", "2000:9940", "move-token helper callee B"},
		{"2000:9c86", "2000:9e42", "pre-wait unlabeled range"},
		{"2000:9d48", "2000:9e1d", "move-token recovery block"},
		{"2000:9b82", "2000:9bc4", "timeout message path"},
		{"2000:9e1e", "2000:9e42", "wait wrapper"}
	};

	private static final String[][] DATA_TARGETS = new String[][] {
		{"2000:0653", "token_timeout_caption_ptr_candidate"},
		{"2000:066e", "token_timeout_tail_string_candidate"},
		{"2000:2f72", "token_wait_result_low"},
		{"2000:2f74", "token_wait_result_high"},
		{"2000:2f76", "token_wait_state_flag"},
		{"2000:34fa", "token_wait_start_time_low"},
		{"2000:34fc", "token_wait_start_time_high"},
		{"2000:46cc", "token_timeout_message_ptr_candidate"}
	};

	@Override
	protected void run() throws Exception {
		String[] args = getScriptArgs();
		File outputDir = args.length >= 1 ? new File(args[0]) : new File("artifacts/ghidra/ecmaint-live");
		if (!outputDir.exists() && !outputDir.mkdirs()) {
			throw new IllegalStateException("failed to create output directory: " + outputDir);
		}

		File report = new File(outputDir, "token-reachability.txt");
		try (PrintWriter out = new PrintWriter(new FileWriter(report))) {
			out.printf("Program: %s%n%n", currentProgram.getName());
			writeDataTargets(out);
			for (String[] range : CODE_RANGES) {
				writeRange(out, range[0], range[1], range[2]);
			}
		}
		println("Wrote " + report.getAbsolutePath());
	}

	private void writeDataTargets(PrintWriter out) throws Exception {
		out.println("Data Targets");
		for (String[] target : DATA_TARGETS) {
			Address address = toAddr(target[0]);
			Data data = getDataAt(address);
			out.printf("%s %s%n", address, target[1]);
			out.printf("- defined data: %s%n", data == null ? "<none>" : data);
			out.printf("- bytes:");
			for (int i = 0; i < 8; i++) {
				out.printf(" %02x", getByte(address.add(i)) & 0xff);
			}
			out.println();
			out.println();
		}
	}

	private void writeRange(PrintWriter out, String startText, String endText, String label) throws Exception {
		Address start = toAddr(startText);
		Address end = toAddr(endText);
		out.printf("%s (%s .. %s)%n", label, start, end);

		Instruction instruction = getInstructionAt(start);
		if (instruction == null) {
			disassemble(start);
			instruction = getInstructionAt(start);
		}

		while (instruction != null && instruction.getAddress().compareTo(end) <= 0 && !monitor.isCancelled()) {
			out.printf("- %s  %s", instruction.getAddress(), instruction);

			Function function = getFunctionContaining(instruction.getAddress());
			if (function != null && instruction.getAddress().equals(function.getEntryPoint())) {
				out.printf("    [function %s]", function.getName());
			}

			Reference[] refs = instruction.getReferencesFrom();
			for (Reference ref : refs) {
				if (ref.getToAddress() != null) {
					out.printf("    [ref %s]", ref.getToAddress());
				}
			}

			for (int i = 0; i < instruction.getNumOperands(); i++) {
				Object[] objects = instruction.getOpObjects(i);
				for (Object object : objects) {
					if (object instanceof Scalar scalar) {
						out.printf("    [scalar 0x%x]", scalar.getUnsignedValue());
					}
					else if (object instanceof Address refAddr) {
						out.printf("    [addr %s]", refAddr);
					}
					else if (object instanceof Register register) {
						out.printf("    [reg %s]", register.getName());
					}
				}
			}

			out.println();
			instruction = instruction.getNext();
		}
		out.println();
	}
}
