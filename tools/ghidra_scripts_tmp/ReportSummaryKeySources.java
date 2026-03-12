// Freeze the raw sources behind the kind-1/kind-2 summary key fields that
// feed the later matcher. This is intentionally small and source-oriented.

import java.io.File;
import java.io.FileWriter;
import java.io.IOException;
import java.io.PrintWriter;

import ghidra.app.script.GhidraScript;

public class ReportSummaryKeySources extends GhidraScript {
    @Override
    protected void run() throws Exception {
        File out = new File(System.getProperty("user.dir"),
            "artifacts/ghidra/ecmaint-live/summary-key-sources.txt");
        out.getParentFile().mkdirs();
        try (PrintWriter writer = new PrintWriter(new FileWriter(out))) {
            writeReport(writer);
        }
        println("Wrote " + out.getAbsolutePath());
    }

    private void writeReport(PrintWriter writer) throws IOException {
        writer.println("Summary Key Source Report");
        writer.println();
        writer.println("Focus:");
        writer.println("- lock down the raw file/scratch sources behind the summary words");
        writer.println("  that feed the later kind-2 matcher");
        writer.println("- especially kind-1 summary +0x0A/+0x06 and kind-2 summary +0x06");
        writer.println();

        writer.println("kind-1 summary emitter (2000:6040..6368)");
        writer.println("- primary fleet branch:");
        writer.println("  - 2000:6158 MOV AX,word ptr [BP + 0xFF3E]");
        writer.println("  - 2000:6160 MOV word ptr ES:[DI + 0x0A],AX");
        writer.println("  - [BP+0xFF3E] is the first word of the loaded fleet record scratch");
        writer.println("  - practical mapping: summary +0x0A <- fleet raw[0x00..0x01]");
        writer.println("  - in preserved one-base case this is 0x0001");
        writer.println();
        writer.println("  - 2000:61E7 MOV AX,word ptr ES:[DI + 0x40]");
        writer.println("  - 2000:61EF MOV word ptr ES:[DI + 0x06],AX");
        writer.println("  - practical mapping: summary +0x06 <- player raw[0x40..0x41]");
        writer.println("    from the active player record");
        writer.println();
        writer.println("- follow-on fleet branch:");
        writer.println("  - 2000:62BA MOV AX,word ptr [BP + 0xFF3E]");
        writer.println("  - 2000:62C2 MOV word ptr ES:[DI + 0x0A],AX");
        writer.println("  - practical mapping: summary +0x0A <- fleet raw[0x00..0x01]");
        writer.println();
        writer.println("  - 2000:62E5 MOV AX,word ptr [BP + 0xFF43]");
        writer.println("  - 2000:62ED MOV word ptr ES:[DI + 0x06],AX");
        writer.println("  - [BP+0xFF43] is fleet scratch offset +0x05");
        writer.println("  - practical mapping: summary +0x06 <- fleet raw[0x05..0x06]");
        writer.println();

        writer.println("kind-2 summary emitter (2000:63D3..6759)");
        writer.println("- primary base branch:");
        writer.println("  - 2000:64EB MOV AX,word ptr [BP + 0xFF76]");
        writer.println("  - 2000:64F3 MOV word ptr ES:[DI + 0x0A],AX");
        writer.println("  - [BP+0xFF76] is base scratch offset +0x02");
        writer.println("  - practical mapping: summary +0x0A <- base raw[0x02..0x03]");
        writer.println();
        writer.println();
        writer.println("  - 2000:6576 MOV AX,word ptr ES:[DI + 0x44]");
        writer.println("  - 2000:657E MOV word ptr ES:[DI + 0x06],AX");
        writer.println("  - practical mapping: summary +0x06 <- player raw[0x44..0x45]");
        writer.println("    from the active player record");
        writer.println();
        writer.println("- follow-on base branch:");
        writer.println("  - 2000:6645 MOV AX,word ptr [BP + 0xFF76]");
        writer.println("  - 2000:664D MOV word ptr ES:[DI + 0x0A],AX");
        writer.println("  - practical mapping: summary +0x0A <- base raw[0x02..0x03]");
        writer.println();
        writer.println("  - 2000:66C4 MOV AX,word ptr [BP + 0xFF7B]");
        writer.println("  - 2000:66CC MOV word ptr ES:[DI + 0x06],AX");
        writer.println("  - [BP+0xFF7B] is base scratch offset +0x07");
        writer.println("  - practical mapping: summary +0x06 <- base raw[0x07..0x08]");
        writer.println();

        writer.println("Matcher consequence (0000:03DF..06AE)");
        writer.println("- direct accept path:");
        writer.println("  - candidate kind-1 summary +0x0A == decoded[0x3558]");
        writer.println("  - this compares fleet raw[0x00..0x01] against a decoded key derived");
        writer.println("    from base-side summary +0x06");
        writer.println("- structural accept path:");
        writer.println("  - candidate kind-1 summary +0x06 is decoded through the sibling");
        writer.println("    helper into a local buffer");
        writer.println("  - decoded local +0x23 == [0x355A], decoded local +0x1F == 4,");
        writer.println("    decoded local +0x0A == 0");
        writer.println("  - this uses fleet raw[0x05..0x06] in the follow-on kind-1 branch");
        writer.println("    and player raw[0x40..0x41] in the primary kind-1 branch");
        writer.println();

        writer.println("Practical one-base inference");
        writer.println("- in the preserved accepted one-base guard-starbase case:");
        writer.println("  - player raw[0x44..0x45] = 0x0001");
        writer.println("  - fleet raw[0x00..0x01] = 0x0001");
        writer.println("  - fleet raw[0x05..0x06] = 0x0001");
        writer.println("  - base raw[0x07..0x08] = 0x0001");
        writer.println("- this is the clearest current reason the one-base fixture survives");
        writer.println("  the matcher even before the helper-decoded semantics are fully named");
    }
}
