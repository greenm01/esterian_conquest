import ghidra.app.script.GhidraScript;

public class HelloScript extends GhidraScript {
    @Override
    public void run() throws Exception {
        println("Hello from Ghidra!");
    }
}
