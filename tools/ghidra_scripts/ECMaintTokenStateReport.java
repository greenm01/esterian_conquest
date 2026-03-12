//@category EsterianConquest

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;
import ghidra.program.model.symbol.SourceType;

public class ECMaintTokenStateReport extends GhidraScript {

	private static final String[][] LABELS = new String[][] {
		{"2000:34fa", "ecmaint_token_wait_start_time_low"},
		{"2000:34fc", "ecmaint_token_wait_start_time_high"},
		{"2000:2f72", "ecmaint_token_wait_result_low"},
		{"2000:2f74", "ecmaint_token_wait_result_high"},
		{"2000:2f76", "ecmaint_token_wait_state_flag"},
		{"2000:46cc", "ecmaint_token_timeout_message_ptr_candidate"},
		{"2000:0653", "ecmaint_token_timeout_caption_ptr_candidate"}
	};

	@Override
	protected void run() throws Exception {
		String[] args = getScriptArgs();
		File outputDir = args.length >= 1 ? new File(args[0]) : new File("artifacts/ghidra/ecmaint-live");
		if (!outputDir.exists() && !outputDir.mkdirs()) {
			throw new IllegalStateException("failed to create output directory: " + outputDir);
		}

		for (String[] pair : LABELS) {
			createLabel(toAddr(pair[0]), pair[1], true, SourceType.USER_DEFINED);
		}

		File report = new File(outputDir, "token-state.txt");
		try (PrintWriter out = new PrintWriter(new FileWriter(report))) {
			out.printf("Program: %s%n%n", currentProgram.getName());
			for (String[] pair : LABELS) {
				writeRefs(out, pair[0], pair[1]);
			}
		}
		println("Wrote " + report.getAbsolutePath());
	}

	private void writeRefs(PrintWriter out, String addressText, String label) throws Exception {
		Address address = toAddr(addressText);
		out.printf("%s %s%n", address, label);
		ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(address);
		int count = 0;
		while (refs.hasNext() && !monitor.isCancelled()) {
			Reference ref = refs.next();
			Function function = getFunctionContaining(ref.getFromAddress());
			String functionName = function == null ? "<no-function>" : function.getEntryPoint() + " " + function.getName();
			out.printf("- %s (%s)%n", ref.getFromAddress(), functionName);
			count++;
		}
		if (count == 0) {
			out.println("- <none>");
		}
		out.println();
	}
}
