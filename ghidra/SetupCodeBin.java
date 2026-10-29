// Runs before auto-analysis of a raw code.bin: marks the entry point at the
// start of .text, which is where a 3DS executable begins, and turns on the
// analyzers that find functions nothing calls directly (vtables, callbacks).
// @category Icarus
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;

public class SetupCodeBin extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address entry = currentProgram.getMemory().getMinAddress();
        currentProgram.getSymbolTable().addExternalEntryPoint(entry);
        createFunction(entry, "entry");
        disassemble(entry);

        setAnalysisOption(currentProgram, "ARM Aggressive Instruction Finder", "true");
        setAnalysisOption(currentProgram, "Function Start Search", "true");
        setAnalysisOption(currentProgram, "Scalar Operand References", "true");
    }
}
