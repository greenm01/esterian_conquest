import ghidra.app.script.GhidraScript;
public class MyScript extends GhidraScript {
    @Override
    protected void run() throws Exception {
        println("It worked!");
    }
}
