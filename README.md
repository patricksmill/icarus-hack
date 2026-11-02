# Kid Icarus: Uprising on PC

The goal is to play Kid Icarus: Uprising (USA, 0004000000030100) on a PC with a
mouse and keyboard, on one screen of any shape, without changing how the game
plays.

It is not a standalone native port. The game runs in a fork of the Zakuro 3DS
emulator, with its code recompiled ahead of time by 3dsrecomp. A few functions
are hooked so the fork can change what they do; everything else runs as the
game wrote it. Each hook is checked against unhooked play over the same frames,
so gameplay stays the same.

## What works today

### Mouse and keyboard

- **Air battles**: the mouse aims where you point, absolutely.
- **Ground battles**: the mouse turns the camera, relative like a PC shooter.
- No touch screen involved.

### One screen

Everything you need in battle is drawn on the top screen by the game's own UI
code, so it stays sharp at any resolution:

- narration and subtitles
- character portraits
- dialogue illustrations and boss cards
- hearts, item box, power icon, held item and pickup popups
- the pause menu, Options and its pages, and the dialogs over them

The bottom screen keeps its native drawing and touch controls. Menus that
support the mouse move onto the top screen too, with proportional artwork,
grouped lists and compact hints.

### Widescreen at any shape

- A wider view, not a stretched one, at 16:9, 21:9, 32:9, or a window dragged
  to any size.
- No pop-in at the new edges; the HUD keeps its shape and sits by the window's
  edges.
- Aiming is correct across the whole width.

### Menus with a mouse

- The option under the pointer lights; a click uses it; over nothing, nothing
  is lit.
- Keys and gamepads get the game's own cursor back.
- Works in the pause menu, Options, dialogs, file select, chapter select, the
  main menu, the Solo hub, Arms Altar, Practice Range, Idol Toss, Treasure
  Hunt, the Vault, the Armory, and the Fiend's Cauldron.
- Armory lists scroll with the wheel; Shift + wheel scrolls horizontally.
- Click-and-hold and dragging stay native touch.

### Performance

- 60 FPS patch included (`dist/60fps/code.ips`).
- Smaller HUD and reticle.
- CPU overclock for heavy scenes.
- Land battles run at full speed.

## Known limits

- Online (Together) mode is out of scope for now.
- Windows mouse testing is still an acceptance check; the headless checks pass.
- Scripted menu overlays need the 3D slider off, and dense stereo menu boards
  fall back to native rendering.
- A few controls still need a held touch: Control Configuration and the
  Air/Land Battle sliders. The Music Gallery list does not take the wheel yet.
- The Fiend's Cauldron's bottom ring background and hearts-box frame are still
  being restored.
- Boss coverage: natural boss defeat, and bosses beyond Medusa, Hewdraw and
  Twinbellows, still need checks.
- Picking a different power and using items from the keyboard are next.
- The project's own settings (HUD scale, aim sensitivity, overclock) are still
  environment variables; moving them into Zakuro's settings menu is planned.
- Dialog text can occasionally stack in some scenes; the cause is known and
  the fix is planned.

## Running it

You need a decrypted dump of the game. Put it in `dumps/`; never commit or
share it.

Build and run the fork with the game:

```
cargo run --release --manifest-path tools/zakuro/Cargo.toml -p zakuro -- dumps/kiu.cci
```

Recompile the game once from Zakuro's library (the Recompile button) so it runs
at full speed. Esc opens the menu over the game, where the controls, sound and
graphics settings live.

The one-screen features can be turned off for comparison with the original
layout: `ZAKURO_MENUS_TOP=0`, `ZAKURO_NARRATION_TOP=0`, `ZAKURO_MIRROR_BOTTOM=0`,
`ZAKURO_PAUSE_TOP=0`, `ZAKURO_CAULDRON_TOP=0`, `ZAKURO_MENU_HOVER=0`.

## Where to look

- `PLAN.md`: the status of each feature and the work left.
- `analysis/FINDINGS.md`: how the game works, and the addresses behind each
  hook.
- `docs/adr/`: the main design decisions.
- `GLOSSARY.md`: the project's terms.

## Layout

- `dumps/`: your decrypted dump goes here. Never commit or share it.
- `tools/zakuro`: the Zakuro fork, where the hooks and the frontend options
  live.
- `tools/3dsrecomp`: the recompiler, with its patch for Zakuro's ABI.
- `tools/kiutrace`: headless harness for testing scenes from save states
  (input, screenshots, memory, tracing, and each hook on its own).
- `tools/funclist`: exports 3dsrecomp's function list for Ghidra.
- `ghidra/`: headless import, decompile and disassembly scripts for code.bin.
- `scripts/`: float scans, the 60 FPS IPS builder, code diffs, screenshot
  conversion.
- `overrides/`: 3dsrecomp C overrides from the first approach (a placeholder;
  the hooks are in the Zakuro fork now).
- `analysis/`, `trace/`, `build/`, `dist/`: generated output, kept out of git
  because it holds game code or binaries.
