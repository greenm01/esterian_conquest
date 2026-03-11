//@category EsterianConquest

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Comparator;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Set;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Data;
import ghidra.program.model.listing.DataIterator;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionIterator;
import ghidra.program.model.listing.Listing;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

public class ECMaintDumpAnchors extends GhidraScript {

	private static final List<String> TARGET_STRINGS = Arrays.asList(
		"PLANETS.DAT",
		"BASES.DAT",
		"MESSAGES.DAT",
		"RESULTS.DAT",
		"FLEETS.DAT",
		"IPBM.DAT",
		"PLAYER.DAT",
		"CONQUEST.DAT",
		"SETUP.DAT",
		"DATABASE.DAT",
		"RANKINGS.TXT",
		"ERRORS.TXT");

	@Override
	protected void run() throws Exception {
		String[] args = getScriptArgs();
		File outputDir = args.length >= 1 ? new File(args[0]) : new File("artifacts/ghidra/ecmaint");
		if (!outputDir.exists() && !outputDir.mkdirs()) {
			throw new IllegalStateException("failed to create output directory: " + outputDir);
		}

		writeFunctions(new File(outputDir, "functions.txt"));
		writeTargetStrings(new File(outputDir, "target-strings.txt"));
		writeInterestingStrings(new File(outputDir, "interesting-strings.txt"));
	}

	private void writeFunctions(File outputFile) throws Exception {
		List<Function> functions = new ArrayList<>();
		FunctionIterator iter = currentProgram.getListing().getFunctions(true);
		while (iter.hasNext() && !monitor.isCancelled()) {
			functions.add(iter.next());
		}

		try (PrintWriter out = new PrintWriter(new FileWriter(outputFile))) {
			out.printf("Program: %s%n", currentProgram.getName());
			out.printf("Function count: %d%n%n", functions.size());
			for (Function function : functions) {
				out.printf("%s %s%n", function.getEntryPoint(), function.getName());
			}
		}
		println("Wrote " + outputFile.getAbsolutePath());
	}

	private void writeTargetStrings(File outputFile) throws Exception {
		List<StringHit> hits = new ArrayList<>();
		for (Data data : definedData()) {
			String value = extractString(data);
			if (value == null) {
				continue;
			}
			for (String target : TARGET_STRINGS) {
				if (value.equalsIgnoreCase(target)) {
					hits.add(new StringHit(data.getAddress(), value));
					break;
				}
			}
		}

		hits.sort(Comparator.comparing(hit -> hit.address.toString()));

		try (PrintWriter out = new PrintWriter(new FileWriter(outputFile))) {
			out.printf("Program: %s%n", currentProgram.getName());
			out.printf("Target string hits: %d%n%n", hits.size());
			for (StringHit hit : hits) {
				out.printf("%s \"%s\"%n", hit.address, hit.value);
				writeReferences(out, hit.address);
				out.println();
			}
		}
		println("Wrote " + outputFile.getAbsolutePath());
	}

	private void writeInterestingStrings(File outputFile) throws Exception {
		Set<String> keywords = new HashSet<>(Arrays.asList(
			"star", "base", "fleet", "planet", "player", "guard", "bomb", "build",
			"rank", "error", "maint", "conquest"));
		List<StringHit> hits = new ArrayList<>();
		for (Data data : definedData()) {
			String value = extractString(data);
			if (value == null) {
				continue;
			}
			String lower = value.toLowerCase(Locale.ROOT);
			for (String keyword : keywords) {
				if (lower.contains(keyword)) {
					hits.add(new StringHit(data.getAddress(), value));
					break;
				}
			}
		}

		hits.sort(Comparator.comparing(hit -> hit.address.toString()));

		try (PrintWriter out = new PrintWriter(new FileWriter(outputFile))) {
			out.printf("Program: %s%n", currentProgram.getName());
			out.printf("Interesting string hits: %d%n%n", hits.size());
			for (StringHit hit : hits) {
				out.printf("%s \"%s\"%n", hit.address, hit.value);
				writeReferences(out, hit.address);
				out.println();
			}
		}
		println("Wrote " + outputFile.getAbsolutePath());
	}

	private Iterable<Data> definedData() {
		List<Data> dataItems = new ArrayList<>();
		Listing listing = currentProgram.getListing();
		DataIterator iter = listing.getDefinedData(true);
		while (iter.hasNext() && !monitor.isCancelled()) {
			dataItems.add(iter.next());
		}
		return dataItems;
	}

	private String extractString(Data data) {
		String typeName = data.getDataType().getName().toLowerCase(Locale.ROOT);
		if (!typeName.contains("string")) {
			return null;
		}
		Object value = data.getValue();
		if (value == null) {
			return null;
		}
		String text = value.toString().trim();
		if (text.isEmpty() || text.length() < 4) {
			return null;
		}
		return text;
	}

	private void writeReferences(PrintWriter out, Address toAddress) {
		ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(toAddress);
		int count = 0;
		while (refs.hasNext() && !monitor.isCancelled()) {
			Reference ref = refs.next();
			Function function = getFunctionContaining(ref.getFromAddress());
			String functionName = function == null ? "<no-function>" : function.getName();
			out.printf("  <- %s (%s)%n", ref.getFromAddress(), functionName);
			count++;
		}
		if (count == 0) {
			out.println("  <- <no references>");
		}
	}

	private static class StringHit {
		private final Address address;
		private final String value;

		private StringHit(Address address, String value) {
			this.address = address;
			this.value = value;
		}
	}
}
