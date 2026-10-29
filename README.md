# Kid Icarus: Uprising PC port (3dsrecomp + Zakuro)

## Layout

- `dumps/`: your decrypted dump goes here. Never commit or share it.
  - `exefs/code.bin`, `exheader.bin`, and any `.cro`/`.crs`/`.crr` from the RomFS
- `tools/3dsrecomp`: upstream clone, built with verify (`target/release/3dsrecomp`)
- `tools/zakuro`: upstream clone (not built; you run a release on your PC)
- `tools/ghidra_12.2_PUBLIC`, `tools/jdk-25*`: Ghidra and the Java it needs, used by `ghidra/analyze.sh`
- `scripts/scan_floats.py`: quick scan of code.bin for screen-shape float constants and the literal pools that point at them
- `ghidra/analyze.sh` + `SetupCodeBin.java` + `FindProjection.java`: headless import of code.bin (ARM v6, base 0x00100000) and a report of every function that loads those constants, written to `ghidra/reports/`
- `overrides/`: hand-written C overrides passed to `3dsrecomp port --overrides overrides/`

## Order of work

1. Boot the dump in a Zakuro release on your PC. This decides go or no-go: Kid Icarus isn't on Zakuro's compatibility list.
2. `scan_floats.py` and `ghidra/analyze.sh`: find the projection setup, culling, touch-to-aim handler and HUD 2D transform.
3. `3dsrecomp analyze`, then check that each address we find is a function it recognises.
4. Write the overrides one at a time, checking each with `verify`.
5. Fork Zakuro to add mouse capture, the host bridge for mouse and aspect ratio, and the screen layouts.

## Targets

| Feature | Hook | Address |
| --- | --- | --- |
| Widescreen | 3D projection (aspect 400/240) | TBD |
| No edge pop-in | frustum culling | TBD |
| Mouse aim | touch -> reticle/camera | TBD (air: absolute, ground: relative) |
| Smaller HUD | 2D HUD transform | TBD |
