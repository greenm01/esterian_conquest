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

public class ECMaintTokenFlowReport extends GhidraScript {

	private static final String[] TARGETS = {
		"2000:9e1e",
		"2000:9b13"
	};

	@Override
	protected void run() throws Exception {
		String[] args = getScriptArgs();
		File outputDir = args.length >= 1 ? new File(args[0]) : new File("artifacts/ghidra/ecmaint-live");
		if (!outputDir.exists() && !outputDir.mkdirs()) {
			throw new IllegalStateException("failed to create output directory: " + outputDir);
		}

		File report = new File(outputDir, "token-flow.txt");
		try (PrintWriter out = new PrintWriter(new FileWriter(report))) {
			out.printf("Program: %s%n%n", currentProgram.getName());
			for (String target : TARGETS) {
				writeFunctionReport(out, toAddr(target));
			}
		}
		println("Wrote " + report.getAbsolutePath());
	}

	private void writeFunctionReport(PrintWriter out, Address entry) throws Exception {
		Function function = getFunctionAt(entry);
		out.printf("%s %s%n", entry, function == null ? "<no-function>" : function.getName());

		out.println("- incoming references:");
		ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(entry);
		int refCount = 0;
		while (refs.hasNext() && !monitor.isCancelled()) {
			Reference ref = refs.next();
			Function caller = getFunctionContaining(ref.getFromAddress());
			String callerName = caller == null ? "<no-function>" : caller.getEntryPoint() + " " + caller.getName();
			out.printf("  - %s (%s)%n", ref.getFromAddress(), callerName);
			refCount++;
		}
		if (refCount == 0) {
			out.println("  - <none>");
		}

		out.println("- first instructions:");
		Instruction instruction = getInstructionAt(entry);
		int count = 0;
		while (instruction != null && count < 80 && !monitor.isCancelled()) {
			StringBuilder line = new StringBuilder();
			line.append("  - ").append(instruction.getAddress()).append("  ").append(instruction);
			Reference[] outgoing = instruction.getReferencesFrom();
			for (Reference outgoingRef : outgoing) {
				Address to = outgoingRef.getToAddress();
				if (to == null) {
					continue;
				}
				Function callee = getFunctionAt(to);
				String calleeName = callee == null ? "" : " => " + callee.getName();
				line.append("    [ref ").append(to).append(calleeName).append("]");
			}
			out.println(line);
			instruction = instruction.getNext();
			count++;
		}
		out.println();
	}
}
