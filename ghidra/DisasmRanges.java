// Disassembles the given ARM ranges and prints the listing, for reading
// small pieces of code (patches, hooks) without a full analysis.
// args: output file, then start:end pairs in hex
// @category Icarus
import ghidra.app.cmd.disassemble.ArmDisassembleCommand;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.address.AddressSet;
import ghidra.program.model.listing.Instruction;

import java.io.PrintWriter;

public class DisasmRanges extends GhidraScript {
    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        try (PrintWriter w = new PrintWriter(args[0])) {
            for (int i = 1; i < args.length; i++) {
                String[] r = args[i].split(":");
                Address start = toAddr(Long.parseLong(r[0], 16));
                Address end = toAddr(Long.parseLong(r[1], 16));
                w.printf("---- %s..%s%n", start, end);
                for (Address a = start; a.compareTo(end) < 0; ) {
                    Instruction ins = getInstructionAt(a);
                    if (ins == null) {
                        new ArmDisassembleCommand(a, new AddressSet(a, a.add(3)), false).applyTo(currentProgram, monitor);
                        ins = getInstructionAt(a);
                    }
                    if (ins == null) {
                        int word = getInt(a);
                        float f = Float.intBitsToFloat(word);
                        w.printf("%s  .word %08X  (%g)%n", a, word, f);
                        a = a.add(4);
                    } else {
                        w.printf("%s  %s%n", a, ins);
                        a = a.add(ins.getLength());
                    }
                }
            }
        }
    }
}
