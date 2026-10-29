# Plan: top-screen-only play

Goal: in battle, everything the player needs is on the top screen. That covers
narration text, portraits, and any bottom-screen information worth keeping. The
bottom screen can then be hidden, and the mouse never has to touch it. This is
done at the game's own UI layer, not by pasting screen captures, so the text
stays sharp at any resolution and the layout follows the HUD scale.

Addresses are for the USA build (0004000000030100). Findings so far are in
`analysis/FINDINGS.md`.

## Where things stand

| Feature | How | Status |
| --- | --- | --- |
| Mouse aim, air | aim delta at 0x006D26C4 (`zakuro-core/src/aim_mouse.rs`) | working, tested by you |
| Mouse look, ground | camera box collapsed at 0x006D2ED4 | working, tested by you |
| Smaller HUD and reticle | sprite flush 0x0092F408, reticle size 0x007C4BD0 | working |
| 60 FPS | `dist/60fps/code.ips`, 5 words | working, speed verified |
| Recompiled + hooks | 4 hooked functions interpreted, rest native | working, tested by you |
| Overclock for heavy scenes | `ZAKURO_OVERCLOCK`, default 2 | built, untested in heavy scenes |
| Widescreen | projection 0x003901C4, HUD, reticle bounds (from Darkshade's mod) | mapped, not built |
| Stacked dialog text | probably Zakuro's GPU texture cache | to confirm (software rasterizer test) |

## What the dialog looks like to the game

From sprite logging in a Chapter 1 air battle:

- Sprites go into a shared batch (for example 0x1414A8B8): quads of 4 vertices,
  0x18 bytes each, laid out as x, y, z, colour, u, v. They are flushed into a
  GPU command list by 0x0092F408. Top and bottom screens share the batch; the
  screen depends on which render pass is running.
- The dialog line is one quad from caller 0x00A192E4, at (19, 11)-(213, 51) in
  bottom-screen pixels, sampling (0, 0.375)-(0.758, 1) of a texture. So the
  game renders each line into a texture first, which also explains how stale
  text can stack if a cache misses the clear.
- The texture is not set inside the batch. It is bound earlier, through the
  game's material and render state.
- Bottom-screen callers seen so far: 0x00A0F3xx-0x00A0F6xx (frame),
  0x00A11128-0x00A11CE8 (item box), 0x00A13670-0x00A13B68 (special weapon
  icons), 0x00A17E70-0x00A181C0 (background), 0x00A192E4 (dialog line),
  0x00A1950C and 0x00A196C0 (chapter title, hearts counter).

## The approach

There are three ways to get bottom-screen UI onto the top screen. This plan
uses A, keeps B as a fallback, and avoids C.

**A. Move the UI element to the top screen's draw list (preferred).** The game
draws each screen's 2D UI in its own pass, probably as a list of UI objects per
screen. If the dialog object is moved, or added, to the top screen's list, the
game draws it there with its own textures, animations, fades and alpha. We
then only adjust its position in the flush hook, from bottom-screen
coordinates to wherever it should sit on the top screen, scaled with the HUD.
This needs the most understanding up front but leaves the least to maintain.

**B. Replay the dialog's draws in the top pass.** Record the dialog's quads and
its texture and material state during the bottom pass, then issue them again
from the top pass. This works without understanding the UI framework. It is
fragile, though: we have to reproduce every bit of render state the dialog
depends on.

**C. Composite in the emulator.** Find the dialog's draws in Zakuro's GPU
(by texture address) and redraw them on the top framebuffer. It is tied to
Zakuro's renderer, hard to move to the port, and the hardest to get right.
Not planned.

## Phase 0: groundwork (small, first)

1. **Shared save states.** Your Zakuro build and my harness now share the same
   core, so F5 save states you make load straight into `tools/kiutrace`. From
   your copy, please save states in: an air battle mid-dialog, a ground battle
   mid-dialog, a boss intro (Twinbellows), a scene using a power or item, and
   the pause menu in battle. Scenes I can't reach headless become testable.
2. **Version control.** Put the project in git, commit the Zakuro fork's
   branch, and record the recomp-abi patch. Every change is then reviewable and
   can be undone.
3. **Stacked text check.** You switch to the software rasterizer briefly. If
   the stacking goes away, it's Zakuro's texture cache (Phase 6).

## Phase 1: map the bottom screen's UI

Make a complete inventory before moving anything:

1. With the harness, log every bottom-screen flush (caller, quads, positions)
   in each saved scene, with screenshots. That produces a table of element,
   caller, region and when it shows.
2. For each element, decide whether it moves to the top screen, stays, or is
   dropped, and how the player uses it (read only, or touched). Draft:

| Element | On top? | Notes |
| --- | --- | --- |
| Dialog text | yes | first target |
| Portraits | yes | second target |
| Hearts counter | yes, small | it's information |
| Special weapon / item icons | yes, small | needs a key to use them (Phase 4) |
| Chapter title | no | decoration |
| Background, frame | no | decoration |
| Power or item selection screens | stays on bottom | opened from a key, shown when needed (Phase 5) |

## Phase 2: understand the UI framework (research)

Questions to answer, in this order:

1. **Who calls the dialog flush?** Walk up from 0x00A192E4 to the dialog's UI
   object: its class, vtable, and draw method.
2. **How is a screen's UI drawn?** Find the pass that draws the bottom screen's
   2D UI and the one for the top's. Is there a per-screen list or manager the
   dialog object belongs to? The top HUD callers (0x009F2F90, 0x009F45FC and
   so on) lead to the top's equivalent.
3. **How are textures and materials bound for a sprite?** Find what runs before
   the dialog's quads go in, and whether that state belongs to the UI object
   (good for A) or to the pass (makes A harder).
4. **How is the line's text rendered?** Find the glyph-to-texture path and its
   clear. This is needed both to move it and for the stacked-text fix.
5. **What does the projection depend on?** Top and bottom use different pixel
   spaces (400 vs 320 wide). Find where each pass sets its 2D projection, so a
   moved element is drawn in top-screen space.

Tools: the harness (call logging at chosen addresses, write tracing, memory
reads, screenshots), the Ghidra decompile (`analysis/all.c`), and your save
states. The output is a section in FINDINGS.md that settles approach A or B.

## Phase 3: dialog text on the top screen

1. Prototype in the harness. Add the dialog's draw to the top pass (A), or
   replay its quad (B). Check with screenshots that the text appears on the top
   screen, sharp, in the right place, and only while a line is showing.
2. Placement: by default, centred near the bottom of the top screen like
   subtitles, scaled by the HUD scale. Adjustable later.
3. Stop drawing it on the bottom screen, so it isn't shown twice when both
   screens are visible.
4. Readability: if the line's texture has no background of its own, add a
   translucent backing strip, or keep the game's own text box.
5. Port the prototype into the Zakuro fork (`aim_mouse.rs`, which by then
   should be renamed to something like `kiu.rs`). Interpret any new hooked
   functions the same way as the others.
6. Test across scenes from your save states: air, ground, boss, back-to-back
   lines, and lines that interrupt each other.

## Phase 4: portraits, then the rest

1. Portraits, done the same way. They animate (expressions, slide-ins), so
   check that the animation still plays on the top screen.
2. Hearts counter and weapon/item icons, small, anchored to a corner and
   scaled with the HUD.
3. Anything else Phase 1 marks as needed.

## Phase 5: playing without the bottom screen

1. **Keys for touch actions.** Using a power or an item means tapping the
   bottom screen. Add bindings in the fork that tap the right spot while the
   mouse is captured. The positions come from Phase 1.
2. **Automatic bottom screen.** Add a layout option: in battle with the mouse
   captured, show only the top screen. In menus, or when a power or item
   screen opens, bring the bottom screen back. The trigger is either the mouse
   capture state or a game state flag found in Phase 2.
3. **Settings.** Move the environment variables (sensitivity, HUD scale,
   overclock) and new options (dialog position, bottom screen behaviour) into
   Zakuro's settings menu.

## Phase 6: stacked text fix

If Phase 0 points at the texture cache: find how Zakuro tracks writes to
texture memory, and why the game's clear of the line texture is missed. It is
either a GPU memory fill or a CPU write from recompiled code. Fix it so the
cache invalidates on that write. If it's a general bug, it's worth reporting
upstream too.

## Phase 7: widescreen (after this)

Already mapped. Projection scale at 0x003901C4 (copy and scale row 1),
culling, HUD squeeze in the sprite flush (which also has to place the new
top-screen dialog), and reticle bounds in 0x006D14F0 and 0x006D269C. With the
mouse, the aim mapping has to account for the wider view. This comes after the
UI move, so all top-screen UI is anchored correctly from the start.

## How each step is checked

- Headless first: harness screenshots before and after, from your save
  states, at both 30 and 60 FPS, plus a check that game speed is unchanged.
- Then a Windows build for you, with what to look for.
- Nothing ships that I haven't seen working headless, unless I say so.

## Risks and unknowns

- The UI framework may not let objects move between screens, for example if
  each screen's UI is one baked layout. Then approach B is the way.
- Text textures may be shared or rewritten mid-frame, which affects B.
- New hooks add interpreted functions. They should be small, and we check
  frame rate after each.
- Together (online) mode and menus are out of scope for now.

## Decisions (from you)

1. Dialog sits bottom centre on the top screen, like subtitles.
2. The bottom screen hides automatically in battle while the mouse is captured.
3. Powers use the 1 key. 1 is bound to ZL in your settings, so it uses powers
   only while the mouse is captured and stays ZL otherwise.
