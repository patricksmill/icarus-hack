// Creates the functions 3dsrecomp found that Ghidra's own analysis missed,
// from a list of "address arm|thumb" lines, then writes the decompiled C of
// just those new functions in the same format as DumpDecomp.
// @category Icarus
import ghidra.app.cmd.disassemble.ArmDisassembleCommand;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;

import java.io.PrintWriter;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.List;

public class ImportFunctions extends GhidraScript {
    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        List<Function> created = new ArrayList<>();
        for (String line : Files.readAllLines(Paths.get(args[0]))) {
            String[] parts = line.trim().split("\\s+");
            if (parts.length < 2)
                continue;
            Address at = toAddr(Long.parseLong(parts[0], 16) & ~1L);
            if (getFunctionAt(at) != null)
                continue;
            if (getInstructionAt(at) == null) {
                clearListing(at);
                new ArmDisassembleCommand(at, null, parts[1].equals("thumb")).applyTo(currentProgram, monitor);
            }
            Function f = createFunction(at, null);
            if (f != null)
                created.add(f);
        }
        println("created " + created.size() + " functions");

        DecompInterface decomp = new DecompInterface();
        decomp.openProgram(currentProgram);
        try (PrintWriter w = new PrintWriter(args[1])) {
            for (Function f : created) {
                if (monitor.isCancelled())
                    break;
                DecompileResults r = decomp.decompileFunction(f, 20, monitor);
                w.printf("== %s %s%n", f.getEntryPoint(), f.getName());
                w.println(r.decompileCompleted() ? r.getDecompiledFunction().getC()
                        : "// decompile failed: " + r.getErrorMessage());
            }
        }
    }
}
