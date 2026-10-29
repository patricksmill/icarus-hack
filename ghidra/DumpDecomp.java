// Writes the decompiled C of every function to one file, each headed by
// "== <entry> <name>", so the whole game can be searched with grep.
// @category Icarus
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;

import java.io.PrintWriter;

public class DumpDecomp extends GhidraScript {
    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        String out = args.length > 0 ? args[0] : "decomp.c";
        DecompInterface decomp = new DecompInterface();
        decomp.openProgram(currentProgram);
        int done = 0, failed = 0;
        try (PrintWriter w = new PrintWriter(out)) {
            for (Function f : currentProgram.getFunctionManager().getFunctions(true)) {
                if (monitor.isCancelled())
                    break;
                DecompileResults r = decomp.decompileFunction(f, 20, monitor);
                w.printf("== %s %s%n", f.getEntryPoint(), f.getName());
                if (r.decompileCompleted())
                    w.println(r.getDecompiledFunction().getC());
                else {
                    w.println("// decompile failed: " + r.getErrorMessage());
                    failed++;
                }
                if (++done % 5000 == 0)
                    println(done + " functions");
            }
        }
        println("wrote " + done + " functions (" + failed + " failed) to " + out);
    }
}
