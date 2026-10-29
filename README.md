# Kid Icarus: Uprising on PC

The goal is to make Kid Icarus: Uprising (USA, 0004000000030100) play well on
a PC with a mouse and keyboard, on one screen of any shape:

- **Mouse aim and look**: absolute aiming in the air, relative camera on the
  ground, with no touch screen involved.
- **One screen**: everything needed in battle (narration, portraits, hearts,
  item box, power icon, pause menu) drawn on the top screen by the game's own
  UI code, so it stays sharp at any resolution. The bottom screen can then be
  hidden.
- **Widescreen at any aspect**: a wider view, not a stretched one, with no
  pop-in at the edges, the HUD the right shape and by the window's edges, and
  aim that is correct across the whole width. The window can be any size.
- **60 FPS**, plus a smaller HUD and reticle, and a CPU overclock for heavy
  scenes.

It is not a standalone native port. The game runs in a fork of the Zakuro 3DS
emulator (branch `kiu-mouse`), from code.bin recompiled by 3dsrecomp
(branch `kiu`). A few hooked functions run in Zakuro's interpreter so the fork
can change what they do, and the rest runs native. Gameplay must stay the
same: each hook is checked against unhooked play over the same frames.

Out of scope for now: online (Together) mode and the menus outside battle.

`PLAN.md` has the status of each feature and the work left.
`analysis/FINDINGS.md` has the addresses and how the game works.

## Layout

- `dumps/`: your decrypted dump goes here. Never commit or share it.
- `tools/zakuro`: the Zakuro fork, where the hooks and the frontend options live
- `tools/3dsrecomp`: the recompiler, with its patch for Zakuro's ABI
- `tools/kiutrace`: headless harness. Runs the game in Zakuro's core from a
  script (input, save states, screenshots, memory, write tracing, call
  logging, and each hook on its own); see the top of `src/main.rs`
- `tools/funclist`: exports 3dsrecomp's function list for Ghidra
- `ghidra/`: headless import of code.bin (ARM v6, base 0x00100000), the
  function import, decompile and disassembly scripts. Uses
  `tools/ghidra_12.2_PUBLIC` and `tools/jdk-25*`
- `scripts/`: float scans, the 60 FPS IPS builder (`make_60fps_ips.py`),
  code diffs, screenshot conversion
- `overrides/`: 3dsrecomp C overrides from the first approach (only a
  placeholder; the hooks are in the Zakuro fork now)
- `analysis/`, `trace/`, `build/`, `dist/`: generated output, kept out of git
  because it holds game code or binaries
