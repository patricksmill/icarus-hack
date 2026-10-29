// Lists every function that loads a float constant tied to the top screen's
// shape, with the constant and a short decompilation, so the projection,
// culling and HUD setups can be picked out by reading the report.
// @category Icarus
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.symbol.Reference;

import java.io.PrintWriter;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.TreeMap;

public class FindProjection extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Map<Float, String> targets = new LinkedHashMap<>();
        targets.put(400f / 240f, "aspect 400/240");
        targets.put(240f / 400f, "inverse aspect");
        targets.put(400f, "top width");
        targets.put(240f, "height");
        targets.put(200f, "half top width");

        String[] args = getScriptArgs();
        String out = args.length > 0 ? args[0] : "projection_report.txt";
        Memory mem = currentProgram.getMemory();
        DecompInterface decomp = new DecompInterface();
        decomp.openProgram(currentProgram);

        // function entry -> constants it reads
        Map<Address, StringBuilder> users = new TreeMap<>();
        for (Map.Entry<Float, String> t : targets.entrySet()) {
            byte[] pattern = intToLe(Float.floatToIntBits(t.getKey()));
            Address at = mem.findBytes(mem.getMinAddress(), pattern, null, true, monitor);
            while (at != null && !monitor.isCancelled()) {
                for (Reference ref : getReferencesTo(at)) {
                    Function f = getFunctionContaining(ref.getFromAddress());
                    if (f == null)
                        continue;
                    users.computeIfAbsent(f.getEntryPoint(), k -> new StringBuilder())
                        .append(String.format("  %s (%s) at %s, read from %s%n",
                            t.getKey(), t.getValue(), at, ref.getFromAddress()));
                }
                at = mem.findBytes(at.add(4), pattern, null, true, monitor);
            }
        }

        try (PrintWriter w = new PrintWriter(out)) {
            for (Map.Entry<Address, StringBuilder> u : users.entrySet()) {
                Function f = getFunctionAt(u.getKey());
                w.printf("== %s %s%n%s", f.getEntryPoint(), f.getName(), u.getValue());
                DecompileResults r = decomp.decompileFunction(f, 30, monitor);
                if (r.decompileCompleted()) {
                    String c = r.getDecompiledFunction().getC();
                    w.println(c.length() > 4000 ? c.substring(0, 4000) + "\n  ...\n" : c);
                }
            }
        }
        println("wrote " + users.size() + " functions to " + out);
    }

    private static byte[] intToLe(int v) {
        return new byte[] { (byte) v, (byte) (v >> 8), (byte) (v >> 16), (byte) (v >> 24) };
    }
}
