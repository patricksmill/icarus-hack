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
| Land battles at 1x | Zakuro: textures of a buffer written back unchanged stay scaled | fixed, built |
| Pause menu on top | pause screen and dialogs replayed in the top passes, touches mapped | working headless, built |

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
3. **Stacked text check.** Done without you: it reproduces in the harness's
   software renderer, so it isn't Zakuro's texture cache. The line's texture
   holds two lines' glyphs, so it wasn't cleared before the new line was
   rendered. Cause to find in Phase 2 (question 4); the fix is Phase 6.

Status: done. States 1-5 load in the harness; the project, the Zakuro fork
(branch `kiu-mouse`) and 3dsrecomp (branch `kiu`) are committed locally.

## Phase 1: map the bottom screen's UI

Status: callers mapped from your states, see FINDINGS.md "Bottom-screen UI
inventory". Narration is 0x0060DB98 (shadow + text quads from one line
texture), portrait 0x009E3068, band 0x0060DB64.

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

Status, for the narration (approach A works, no replay needed):
- Top and bottom battle UI are drawn by 0x0060D39C (top) and the code around
  0x0060D7E8 (bottom), both through one 2D context (0x14155908 in your
  states), which also holds the narration's message at +0x3124.
- The line is drawn by 0x009E9A14(message, context) from the message's own
  fields: shown flag +0, line +4, x/y +0x14/+0x18, size +0x1C/+0x20, texture
  coordinates +0x24. Its texture is picked by index (0x14) in the context, so
  it draws on whichever screen's pass calls it.
- The fork calls it from the top pass (at 0x0060D60C, registers saved and
  restored around the call), with the line moved to the bottom centre.

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

Status: steps 1, 2 and 5 done, tested headless on your states 1, 3 and 4
(centred, scaled with the HUD, shadow kept). Next: your test, then whether
to keep it on the bottom too (both are shown now), and readability.

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

Status: portraits done. The fork calls the game's portrait drawing
(0x009E2BB4, portraits at *(context+0xC3C), drawn at 1 - the bottom UI's fade
at context+0x2454) from the top pass before the narration. Their flush puts
each speaker in the bottom corner on its side, at ZAKURO_PORTRAIT_SCALE (0.5).
Tested headless on states 1, 3 and 4; expressions follow the bottom screen's.

Hearts, item box and power icon: done by record and replay. The bottom
screen's background drawing (0x00A17F70) calls each one (header strip at
0x00A181CC, item box 0x00A18400, power icon 0x00A18430); the hook keeps the
call's target and arguments, and the next frame's top pass makes the call
again, between the portraits and the narration. Its sprites go under the score
(hearts) or the health bar (item box, power icon), at the HUD scale; the
chapter title's (0x00A192E4) are left off. ZAKURO_MIRROR_BOTTOM=0 turns it off.
Checked over 10 s of play: same score, position and pose as without it.

Side finding: the reticle hook now shrinks only the size handed to its
drawing (0x007C4DC0), not the size the reticle keeps (+0x5C), which may feed
lock-on. Every hook leaves gameplay the same (10 s comparison); only the
bottom screen's decorative background animation drifts.
The game seems to have its own upper-screen captions (CaptionUp_Dynamic
layouts): if its option works, the narration hook can default to off.

1. Portraits, done the same way. They animate (expressions, slide-ins), so
   check that the animation still plays on the top screen.
2. Hearts counter and weapon/item icons, small, anchored to a corner and
   scaled with the HUD.
3. Anything else Phase 1 marks as needed.

## Phase 4b: the pause menu on the top screen

Status: working headless on state 5, built. The bottom battle pass
(0x0060D93C) draws the pause screen with one call, 0x00A01A1C(pause at
ctx+0x243C, ctx, shown) at 0x0060DBB4. The fork replays it in the top pass
after the "PAUSED" label (0x0060D68C, reached only while paused), with the
sprites 1:1 at x+40, what spans the bottom screen (dimming, bands)
stretched to 400, and Resume, Retreat and Remove Display by the top
screen's edges (picked by flush caller, main page only). The dialogs over it (Retreat's) are a context of their own
(0x14A1340C): 2D at 0x00722AF4/0x00722B0C, replayed at 0x00725828; text
through the 3D renderer at 0x00722BA4-0x00722DB8 with a projection for the
bottom screen, replayed at 0x007258A0 (right eye) and 0x007258C0 (left eye)
with the projection's x row times 0.8. Options and its four pages work too.

The frontend lets the mouse go while it is shown and holds it again after,
and in top-only maps a click on the top screen to the bottom screen
(`AimMouse::pause_menu_touch`: 40 px to the left, the edge buttons by their
edges, nothing in the gaps they leave). ZAKURO_PAUSE_TOP=0 turns it off.

Checked: Resume, Retreat (No, Yes), Options and its pages by touch at the
mapped spots, 30 and 60 FPS, 3x hardware renderer. The pause object's memory
is the same after 150 frames with and without it, and 300 frames of play
after Resume give identical screens. Retreat's Yes faults the harness at the
scene change with or without the hook (harness limit).

Cosmetic: on Options' pages, elements the bottom screen anchors to its
edges (the OK panel in Reticle/Camera) still sit 40 px in from the top's.

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

## Phase 7: widescreen, at any aspect

Goal: the top screen fills the window at whatever shape it has (16:9,
21:9, 32:9, or a window dragged to any size), with a wider view rather
than a stretched one, no pop-in at the new edges, the HUD and the moved UI
the right shape and by the window's edges, and the mouse aiming right
across all of it.

How it works: the game keeps drawing into its 400x240 buffers, with a view
k times wider squeezed into them, where k = (window width / height) /
(5/3). The presenter stretches the picture back out to the window, and
every top-screen 2D draw gets squeezed by 1/k so it comes out the right
shape. This is how Darkshade's mod works (FINDINGS, "Darkshade's ... patch"),
with k read every frame instead of fixed at 0.9375 or 0.714.

### Steps

1. **Window shape** (frontend). Add a "Fit window" option to Top screen
   shape: in top-only layout, the top screen's width is
   240 x window width / height, limited to 5:3..32:9. Pass the shape to the
   core every frame (a field on `AimMouse`, like `hud_scale`), so k
   follows a resize on the next frame. 16:9 and 21:9 stay as fixed
   choices. Console (5:3) turns all of this off.
2. **Wider view** (core). First look for where the game camera's aspect is
   set (camera+0x40, the aim's camera +0x2BC; "Still to find" in
   FINDINGS), and multiply it there. If both the projection (0x003901C4)
   and the culling (0x00756754 grows its box by tan(fovy/2) x aspect) read
   that field, one write widens both. Fallback: Darkshade's way. At the
   getter's return (0x003901EC), copy the matrix and scale row 1
   (m[4..7]) by 1/k. The projection is cached at camera+0x6C, so it must
   not be scaled in place. The copies need guest memory, from a page
   Zakuro sets aside, or a 32-slot ring like the mod's. Stereo builds
   both eyes from this projection (0x008D11B0), so they widen with it.
3. **No pop-in** (core). If step 2 didn't cover culling, widen the box in
   0x00756754 by k, along with any other culling that reads fovy or
   aspect. Check: turn the camera with enemies and scenery just past the
   old edge, and nothing should appear late.
4. **Top-screen 2D the right shape** (core). Hook 0x0092E4F8(ctx, screen),
   which every 2D pass starts with: 0 is top, 1 is bottom (seen in both
   UI contexts). The sprite flush then knows each batch's screen. Squeeze
   every top flush's x by 1/k about an anchor: the left or right edge for
   what sits by one, the centre otherwise, using the same rule as
   `shrink_hud`'s anchor. Full-width quads (fades, the pause dim) stay
   full width. The HUD scale, reticle, narration, portraits, mirrored
   hearts and pause menu placements all run first and keep working,
   because the squeeze follows them. The pause click mapping
   (`pause_menu_touch`) undoes the same squeeze. Text drawn through the
   3D renderer with its own projection (dialog text, 0x0090A5B4) gets
   row 1 scaled by 1/k too.
5. **Aim** (core). Correct the reticle bounds at 0x006D14F0's returns
   (0x006D15B4, 0x006D1764) and the turn from reticle x at 0x006D26C0,
   both by k, the way the mod does. Divide the mouse's horizontal
   movement by k so the reticle feels the same speed both ways. Check that
   shots hit enemies at the edges in the air, and that ground camera
   turning is unchanged.
6. **Resolution.** Squeezed into 400 columns, the picture is k times softer
   horizontally (1.4 at 21:9). Raising Resolution already makes up for
   that (for example 7x for a 2560x1080 window), at a GPU cost. If that
   cost matters, add a separate horizontal scale for the top screen to the
   hardware renderer (surface rows, viewports, transfers, crops), set to
   k x Resolution. This is optional; it comes last, only if it's needed.

### Checks

- Harness: a `wide <aspect>` command, and screenshots stretched to the
  aspect for viewing. Use your states for air, ground, boss, pause and
  dialog at 5:3, 16:9, 21:9 and 32:9, at 30 and 60 FPS. Look for: round
  things round (hearts icons, reticle), HUD at the edges, nothing missing
  at the edges, and pause clicks landing.
- No gameplay change at 5:3: same 300-frame comparison as before.
- Windows build for you: drag the window through shapes while playing,
  then air aiming at the edges, ground turning, and the pause menu.

### Unknowns

- Where the aspect is set, and whether culling reads it (step 2 decides
  whether one hook is enough).
- 2D on top not drawn through the sprite batch or the text path; found
  by screenshot review.
- Effects that read the frame as a texture (the glow pass from the
  resolution fix) work in buffer space and should be unaffected.

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
