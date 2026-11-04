# Findings: Kid Icarus: Uprising (USA, CTR-P-AKDE, 0004000000030100)

Addresses are for this dump's `code.bin`, loaded at 0x00100000. Internal
title name "apollo". No CRO modules: all code is in the executable.

## Layout

| Segment | Address | Size |
| --- | --- | --- |
| text | 0x00100000 | 0xB950F0 |
| rodata | 0x00C96000 | 0x9B990 |
| data | 0x00D32000 | 0x31C78 |
| bss | after data | 0x1D20B0 |

3dsrecomp: 49,236 functions, 93.6% of text is code, 2.2% unreached.
Ghidra's own analysis found 28,208; `ghidra/ImportFunctions.java` adds the
rest from `analysis/functions.txt` (exported by `tools/funclist`).

## Projection path (widescreen)

| Address | What | Confidence |
| --- | --- | --- |
| 0x003901C4 | Camera projection getter. Lazily builds the pivoted (screen-rotated) projection at `camera+0x6C` (16 floats), sets flags `0x44` at `camera+8`. With no lens object at `camera+4` it uses the default block at 0x00D4E248 {near 0.1, far 1000, fovy 60°, aspect 4/3}; otherwise calls the lens's virtual at vtable+8 to fill the matrix. | high |
| 0x00474BB8 | Trig routine (sin/cos/tan family, 2/π range reduction). Called as tan(fovy/2). | high |
| 0x008D108C | `Frustum(m, l, r, b, t, n, f)` in s0..s5: builds a 4×4 frustum, m[14] = -1. | high |
| 0x008D0C38 | Pivot a projection for the rotated LCD (modes 0–4). | high |
| 0x008D11B0 | Stereo camera: from a base projection and the 3D slider, builds left/right eye projections with Frustum + Pivot. Matches the SDK's ulcd StereoCamera. | high |
| 0x00916234 | Per-frame camera submit: copies view (3×4) and projection (4×4), un-pivots the projection and passes it to 0x008D11B0. | high |
| 0x00911288 | Calls 0x00916234 with the camera's view and `FUN_003901C4(cam)`. | high |

The pivoted projection has the horizontal (400 px) axis in row 1:
`m[1] = cot(fovy/2)`, `m[4] = -cot(fovy/2) / aspect`. Widening the view
means lowering |m[4]| (or raising aspect) before the stereo code runs.

The literal 400/240 (5 copies) is **not** the projection: the three
functions that read it are animation timers (t × 1/0.6 s).

Battle camera, from state 1: 0x00E14BEC, its lens at +4 0x00E14BB4
(perspective, vtable 0x00D20080): near 3.5 (+8), far 40000 (+0xC), fovy
0.785 (+0x10), aspect 1.6667 (+0x14). Two orthographic lenses (vtable
0x00D200B0) sit at 0x00E14BCC (top, -200..200 x -120..120) and 0x00E14DAC
(bottom, -160..160). Nothing writes the aspect during play; writing it and
clearing bit 2 of camera+8 rebuilds the projection with it, and the view
widens (culling and the aim ray with it). Callers of 0x003901C4 each
frame: 0x00360168 (aim ray, twice), 0x007C48EC (reticle), 0x00911698 and
0x00911304 (render submit). The ground turn from the reticle scales with
atan(tan(fovy/2) x aspect).

## Game camera fields

`FUN_0042F534(mgr, 0, 1)` returns the active game camera. Seen in
0x00756754 (visibility bounds): fovy at `+0x3C`, aspect at `+0x40`.
That function expands a box by tan(fovy/2) × dist × aspect, so it is
culling-related and needs widening along with the projection.

## Input services present

`hid:USER` (touch, buttons), `ir:USER` (Circle Pad Pro), `cam:u` (AR
cards). The last two are possible gaps in Zakuro.

## Darkshade's C-Stick & Gyro patch (USA v172), as a map

`patch/` holds it. It replaces code.bin outright and appends 0x1F2000 bytes
of new code at 0x00D64000+ (exheader enlarged). `scripts/diff_code.py`
diffs; `analysis/mod/` has the diffs and a disassembly of the widescreen
code (`w169_disasm.txt`). 16:9 vs 21:9 differ only in the scale constant
(0.9375 vs 0.714 = native 5:3 ÷ target aspect) at 0xF52168 / 0xF52410.

Widescreen hooks (on top of the base patch):

| Hook | Original | Mod code | What it does |
| --- | --- | --- | --- |
| 0x003901EC | `pop {r4,r5,pc}` at end of projection getter | 0xF52000 | If the returned matrix is a sane perspective (m[14] = -1, m[15] = 0, all finite), copy it to a 32-slot ring and scale row 1 (m[4..7], horizontal) by the constant; return the copy. Confirms 0x003901C4 is *the* projection hook. |
| 0x0092F408 | entry of 2D sprite-batch flush | 0xF52430 → 0xF52174 | Only for 14 whitelisted callers (table at 0xF52424; six stubs 0xF52468.. pass the call site): squeezes each quad's x (stride 0x60 per quad, 4 verts × 0x18) about the centre so HUD isn't stretched; edge-anchored elements handled separately (160.0 constant). |
| 0x006D15B4, 0x006D1764 | returns of 0x006D14F0 (reticle bounds) | 0xF52568 → 0xF524C8 | Re-centres reticle bounds on the wider screen using aim+0xE0/+0xE8. |
| 0x006D26C0 | `bl 0x00B12218` in 0x006D269C | 0xF526F8 → 0xF525BC | Corrects the turn amount from reticle x for the wider view. |

Base-patch hooks worth knowing (all in `analysis/mod/base_hooks.txt`):
0x006D2E30–0x006D2F3C wrap the aim update calls; 0x006D1E20/0x006D2050 in
0x006D1FF0; 0x006D27DC; 0x006D2B5C–0x006D2BE4 in 0x006D2ABC; 0x006CF460,
0x006CF7DC; 0x001DCE2C–0x00214834 and 0x0024C270–0x00257F50 (input and
control-settings code, to be read).

## Aim (player reticle) controller

Per-frame update `0x006D2D8C(aim)`:
1. vtable+0x2C(aim): likely the input read (touch/stick → reticle). **To identify.**
2. `0x006D14F0`: reticle free-movement box from aim+0xE4 (half width?),
   +0xEC (half height?) and the state at +0x81 (ground/air?).
3. `0x006D1FF0(aim, out, cam)`: auto-centre / lock-on modes 1–4 at aim+0x208.
4. `0x006D269C(aim, out, cam)`: applies camera turn.
5. `0x006D2ABC`, `0x006D2820`.

Reticle position: aim+0xA4 (x), aim+0xA8 (y). aim+0xD5 = 1 when not the
local player. Game camera (`aim+0x8C → +0x2C`): fovy at +0x2B8, aspect at +0x2BC.

## Still to find

- the lens virtual that fills the projection (camera+4, vtable+8)
- where the game camera's aspect (+0x40) is set
- touch → reticle / camera handler
- HUD 2D projection

## Frame rate (60 FPS patch: `dist/60fps/code.ips`, `scripts/make_60fps_ips.py`)

- Vsync manager singleton (`FUN_003B6FE4`, global 0x00D63824): +0x50 frame
  rate (30), +0x54 swap interval (2), set once in 0x0048A794. Present/wait in
  0x0091038C: waits until interval + leftover vblanks passed; leftover
  (+0xA8) kept as parity `& 1` at 0x0091041C, which at interval 1 forces a
  2-1-1 pattern (40 FPS). +0xB0 = vblanks the last present covered.
- Update pacing 0x00828E34: updates = leftover + vblanks >> 1
  (0x00828E8C); 0x00828D68 sets the frame-count singleton's (0x00E174E4)
  base frames +0x20 to the literal 2.0 at 0x00828E30 each update.
- `ActorTaskUpdate` (0x00A3DB84) turns frames into the frame info object
  (+0xC dt = frames/60, +0x18 frames) that every actor tick receives.
- Patched: 60/60 frames drawn on the old-3DS cycle budget (no overclock
  needed); strafing and firing for 1.5 s matches the unpatched game frame
  for frame.

## Training HUD

Air and land Practice Range both flush the defeat and distance counters at
`0x0060D66C`: one batch of 11 quads spanning both bottom corners. It is
separate from the normal battle HUD callers. The generic widescreen fallback
shrunk each background slice around its own centre, opening gaps, and did
not apply the HUD scale. `place_training_hud` now scales every slice and
digit around its panel's bottom corner (0,240 or 400,240), with widescreen
correction on x. Headless screenshots checked in both modes at 16:9 and
21:9 with HUD scale 0.7; a unit test checks shared slice edges stay joined.

## Dialogue illustrations and boss panels (October 31)

The current desktop slot 1 is the Chapter 3 air scene from the Hewdraw
screenshot. Run 60 refreshes from it to reach "Like, this guy?" and the blue
sprite. The saved fixture is `trace/ui-probe/hewdraw-blue.state`.

- `0x00A17F70` draws the battle's bottom UI. The illustration branch calls
  `0x00A0F734` at `0x00A18244`, handing it the bottom UI, 2D context, pass
  number and fade. It draws a black center band (`0x00A0F814`), zero to four
  illustration sprites (`0x00A0F9E8`), then black bands/borders (return to
  `0x00A18248`). Only the illustration flush is mirrored.
- The bottom UI's `+0x200` is the image-slot mask. Four `0x24`-byte slots
  start at `+0x204`: image id/flags, x/y at +4/+8, UV offsets at +0xC/+0x10,
  width/height at +0x14/+0x18, scale at +0x1C, alpha at +0x20. Hewdraw's slot
  is x 112, y 60, width 24, height 31, scale 4: its quad is 96 x 124, not
  the entire 320 x 130 band. Fit the union of active image rectangles to
  retain multi-sprite arrangements and the original aspect ratio.
- Hearts occupy y `(40..90) * hud_scale` on top. Images start at
  `98 * hud_scale`, end eight HUD pixels above the visible portrait artwork,
  and sit eight HUD pixels from the right edge. Pit's visible head starts
  about 64 pixels into the portrait's 256-pixel tile. Transparent tile
  padding is excluded from the reserved portrait height.
- `0x00A18204` calls `0x00A0E744`, drawing the boss artwork and baked-in
  title in two tiles, x 0..256 and 256..320, y 0..240. Reveal placement is
  128 x 96 HUD pixels, centered at y 40. Player-health visibility
  (`context+0x31BC & 1`) selects the combat layout: clip both tiles and UVs
  to y 0..64, then place the title at y 8, 160 x 32 HUD pixels. Preserve
  their common x transform so the texture seam stays joined.
- `0x00A18224` calls the boss overlay/effect drawing `0x00A14320`.
  `0x00A18130` calls the alternate text drawing `0x00A15C90`; its first
  full-screen black flush (`0x00A15D3C`) is omitted on top. Those paths use
  the boss region. No separate boss-health draw appeared in the inspected
  Twinbellows introduction/combat fixtures. Other bosses and natural defeat
  still need coverage.
- Extras now replay at `0x0060D66C`, after all top battle-HUD branches.
  `0x0060D60C` can be skipped during a reveal. The pass keeps its current
  context in r4; capture that rather than relying on a previously drawn
  score, especially across state loads. Bottom passes clear missing calls;
  recordings are valid for the next top UI pass, independently of the number
  of intervening vblanks. Retain calls for both eyes: the game submits the
  shared UI list to each. Allow twelve refreshes after loading a fixture so
  captures include a completed replay, not only the initial cold frame.
- `0x0060D66C` is also the native training-HUD flush return. Restrict its
  corner placement to native draws; otherwise a replayed narration line
  takes that branch and shifts right. A regression test exercises the real
  sprite hook with narration at this address across HUD scales/aspect ratios.
- Restore the recorded native material selectors around each replay and
  mark their bindings dirty. Suppress decorative bands by discarding their
  pending sprite batch, leaving dirty material state for the next visible
  draw, rather than submitting off-screen degenerate quads.
- Sprite flush flag `0x08` (context +0x50) also writes the GPU scissor
  (registers 0x65-0x67) from context +0xFC/+0x100, packed in framebuffer
  coordinates computed for the bottom screen. The power panel's highlight
  flush (`0x00A116B0`) sets one around the cards, so on the top screen the
  icons and counters after it were clipped away. Mirrored replays move that
  rectangle with the quads (`place_scissor`) and restore it on return.
- Chapter 4's rail (texture `0x04518780`, ETC1A4) is a geometry-shader tube:
  28 centreline vertices become 360 triangles. Its setemit instructions use
  the winding bit (22), which swaps a completed triangle's first two
  vertices; ignoring it culled half the outer faces and drew inner ones,
  giving the opaque, thorny look. The stage-4 combiner then gives
  `tex0.a * 0.898` alpha, i.e. the translucent tube seen on hardware.

Validation artifacts are under `trace/ui-probe`: twenty-four software layouts
(four scenes, three aspect ratios, two HUD scales), 2x hardware captures,
both stereo eyes, and isolated image/boss removal fixtures. All 23
`aim_mouse::tests` pass. After 180 identical firing refreshes per variant in
Hewdraw and Twinbellows combat, the player-health and score objects match
byte-for-byte; all top pixels outside the mirrored UI match. The bottom
screen's decorative background animation can drift, as with earlier replay
hooks. `kiutrace` supports `mirror 0/1`, `stereo 0..1`, and right-eye output
from `shotscaled` for these checks.

## Bottom-screen UI inventory (from your save states, PLAN phase 1)

Sprite flush callers (return addresses into 0x0092F408), by element:

| Caller | Element | Screen | Notes |
| --- | --- | --- | --- |
| 0x0060DB64 | narration band, 0..320 x 0..56 | bottom | dark strip behind the text |
| 0x0060DB98 | **narration text**, 2 quads | bottom | shadow (colour BF000000, +1,+1), then text (FFFFFFFF), same texture region: one texture holds the whole rendered line |
| 0x009E3068 | **portrait**, one 256x256 quad | bottom | inside 0x009E0000.. which `top_hud` treats as top HUD; only skipped by the height check, tighten it |
| 0x00A192E4 | chapter title text | bottom | not the narration |
| 0x00A1950C, 0x00A196C0, 0x00A19748, 0x00A181D0 | hearts counter and icons | bottom | |
| 0x00A11128-0x00A11CE8 | power panel (was listed as the item box) | bottom | three boxes, the current power in the middle one, 49..87 x 202..240 |
| 0x00A13A74, 0x00A13B68 | power / special weapon icon | bottom | |
| 0x009EA550, 0x009EA800, 0x009EAB24 | "You got treasure!" popup: 9-slice box, the item's icon, the two text lines | bottom | 23..301 x 118..191, slides in from the left, about 200 frames, fades out |
| 0x00A17E70-0x00A181C0, 0x00A0F3xx-0x00A0F6xx | background, frame | bottom | |
| 0x0060D68C, 0x00A0D22C-0x00A0E338, 0x00A04710-0x00A04F00, 0x00A01A4C | pause screen | top label, bottom menu | |
| 0x00A0F9E8 | dialogue illustration sprites | bottom | 0x00A0F734, called at 0x00A18244; omit its black bands/borders on top |
| 0x00A0E9D8, 0x00A0EA60 | boss artwork and baked-in title | bottom | two texture tiles, 0..256 and 256..320, both 0..240 vertically; draw call at 0x00A18204 |
| 0x00A182E4 | speakers during boss/background drawing | bottom | call to the normal portrait renderer, not boss artwork |

Pause screen, from state 5: the bottom battle pass 0x0060D93C calls
0x00A01A1C(ctx+0x243C, ctx, shown) at 0x0060DBB4 when 0x00B52774() says
paused; it draws the dimming (0x00A0D22C, 0..320 x 0..240), Resume
(0..148 x 55..111), Retreat (172..320), Options (35..285 x 124..144), loot
and Remove Display, and Options' pages. The top pass 0x0060D3EC calls
0x00A0E6D8(ctx+0x243C, ctx), the "PAUSED" label, at 0x0060D688 only while
paused. The battle bottom elements (hearts, item box) are still drawn under
it. The pause object's state is at +4 (0 closed).

Dialogs (Retreat's "Quit and return to the goddess?") are their own 2D
context (0x14A1340C in state 5). Bottom pass 0x00722A50: 0x00721C64 dimming
and box, 0x00721484 buttons, then the text through the 3D renderer (pointer
at 0x00F2DC9C): 0x00909D54 lights, 0x0090A5B4(renderer, projection 4x4, view
3x4), 0x0090FFA0, 0x00909ACC(text) twice, 0x0090A6A4. The bottom projection
is rows (0, -1/120, 0, 1), (-1/160, 0, 0, 1), (0, 0, 1, 0), (0, 0, 0, 1);
the top pass 0x007255AC uses -1/200 for its own. Top passes draw into both
eyes' buffers: 0x00916588(.., 2) switches to the right eye's (0x1F000000),
(.., 0) back to the left (0x1F080000, the one shown with the slider at 0).
2D sprites go to both through the batch, 3D-renderer text has to be drawn
once in each.

Quad positions are in the element's own space (the narration's text starts at
0,0), placed on screen by a transform elsewhere, not baked into the vertices.

Stacked narration: in your state 1 the line's texture region holds two lines'
glyphs, drawn over each other, so the line texture was not cleared before the
new line was rendered into it. Same with the 60 FPS patch undone afterwards
(the texture was already drawn). Cause to find in the text rendering path
(PLAN phase 2, question 4).

Pickup popup, from your state 8 (6.state of 29 Oct, the Sagittarius Bow):
the popup is its own object at the bottom UI's +0xDC, drawn by
0x009EA06C(popup, ctx), which only reads it (no timers advanced in the
draw). The background drawing 0x00A17F70 calls it at 0x00A18368 in a
normal battle frame, after the portraits, once 0x00B524D8() says so; two
other paths (0x00A18094, and a tail call at 0x00A18030) are for the
bottom UI's other states (+0 bits 0x1000, 0x100000), not seen yet.
The same call draws the item held (used with L, the reticle yellow while
one is): a tab at 286..320 x 206..240 (flush 0x009EAC80), then the icon,
289..321 x 209..241, flushed by a tail call, so the flush returns where
0x009EA06C's call does (0x00A1836C, or the top pass when replayed).

## Powers (miracles, `MRCL` in the game's names)

Found by tapping the power panel in state 1 (Health Recovery, 3 uses) and
tracing what changed.

- Power panel: ctx+0xB64, in the bottom battle 2D context (0x14155330 in
  state 1, the battle UI object 0x00EE3098's +0x140). Words: +0 flags
  (0x800000: no power can be used now), +4 touch state (0x40000000: a power
  asked for, 0x80000000 with it; 0x28000000 busy), +8 command, +0x118 the
  current power's place in the list, +0x184 the list's length, +0x188 the
  list (bytes naming entries). Entry e, 8 bytes at +0x11C + e*8: uses left
  (byte), +1 a byte 0x006DFF2C checks, +2 id (u16, Health Recovery 0x95),
  +5 1 while it can't be used now (Health Recovery at full health).
- Touch: 0x00A129F0 (from the bottom pass at 0x0060DCD8) on a short tap of
  a box (left, middle, right: current - 1, current, current + 1), if it has
  uses and the panel isn't busy, sets +4 |= 0xC0000000, +0x19A = the place,
  +0x19B = the box, +0x1A0 = 0.1, and plays sound 0xB.
- Update: 0x00A17A10(panel), from 0x003EF664. Refreshes +0x121 for the three
  shown, then, if +4 has 0x40000000, the panel isn't locked and the place's
  power can be used, writes the command (keeping `& 0x007E7FEE`):
  `id << 23 | place << 17 | box << 15 | 1` (0x4A808001 for Health Recovery
  in the middle box), clears +4's top two bits and sets +0x19A to 0xFF. If
  it can't, the request stays set.
- Command: 0x00A30D00(UI object, 0 while playing) handles bit 0 with
  0x003636F8(0x00E3F728, player, id) (player able, flags 0x44 clear, uses
  left), then 0x00A17828 for the panel's feedback; the word is cleared after
  (0x00A317A0). Below: 0x0038A3BC(0x00E40978, player, id) ->
  0x003A4CE4 (refuses while running) -> 0x003CC344 -> 0x005A734C, the
  inventory count (0x00E40984 for Health Recovery: 3 -> 2).
- The fork's power key writes that command at 0x00A30D00's entry, when a
  touch of the middle box would ask for it (not locked, not busy, uses
  left, usable now).
- A battle is shown by the top battle pass (0x0060D39C; the fork's hook at
  0x0060D60C inside it): every frame in states 1-5, never on the title
  screen or file select. Paused, it still runs.


## Menu input (pause screen, Options, dialogs)

Found on state 5 with call logging and disassembly (PLAN phase 4c).

- Input record: 0x00A3115C(record, ...) from the input singleton
  (0x002625B8(): +0x2E touching, +0x24/+0x28 the pixel, +8/+0x14 buttons):
  x, y, touching (byte +8), began now (byte +9), key (signed byte +0xA, -1
  none). Keys, from the harness: 2 UP, 3 DOWN, 5 B, 6 SELECT, 8 X, 9 Y,
  10 L and ZR, 11 ZL; 0 and 1 are the other two direction bits (LEFT and
  RIGHT gave none on the main page), 4 button bit 0x100 (A, not seen: B
  had closed the menu). "Began
  now" is set on the touch's first frame only, and that is when an option
  is used (the stylus's press, not its release).
- 0x00A31788 -> 0x0060DC7C(ctx, playing, key, touching, began, x, y): while
  paused (0x00B52774), the key goes to 0x00A0C5A8(pause, key) at
  0x0060DD0C (r5), then the touch to 0x00A01DA0(pause, touching, began, x,
  y) at 0x0060DD24 (r8, r6, r7, y in s16 as an int), the pause screen in
  r9. 0x0060DD28 is after both.
- Pause screen (ctx+0x243C): +4 state (2 main page, 7 Options, 4 closing),
  +0x678 Options' page, +0x6EC the page being gone to, +0x67C the option
  picked, drawn lit; -1 draws none lit (main page, Options' pages, both
  screens), and keys handle it only on pages where the game uses it.
  Main page options: 0 Resume, 1 Retreat, 2 Options, 3 Remove Display.
- 0x00A01DA0 hands a touch to 0x00A0867C (main page) or 0x00A06188
  (Options, when +0x678 == +0x6EC). Both hit test, then at 0x00A08820 (r1)
  and 0x00A06384 (r4) have the option; off every option they return before.
  A different option is picked and plays 0x0075D420(1), the cursor's sound;
  only a touch that began now uses it (on Options' lists, one already
  picked: the first tap picks, the second uses).
- Options' pages: 0-2 lists (0 the back arrow, 1.. the rows), 3-4 choices,
  5 Control Configuration (0x00A0ACF0, drags what it holds), 6
  Reticle/Camera (0x00A0B50C; sliders in the rows above y 70 set by a held
  touch, values at +0x698; starts with -1 picked), 7 Volume (0x00A0C2DC; a
  held touch at x 121..288 of a row sets its slider, values at +0x6AC).
- Dialogs, 0x007226C4(dialog), called from 0x00724738 (Retreat's: 0x14A1340C
  in state 5): only a touch that began now picks a button, which uses it.
  +0x28C how many buttons, +0x2A8 their boxes (4 shorts: left, right, top,
  bottom, inclusive; Retreat's No 36..155, Yes 173..292, 154..188), +0x290
  which cancel, +0x294 the one picked (-1 none; a direction then picks 0).
  The picked one read into r6 at 0x00722754; a change after lights it and,
  at 0x007229E4, plays the cursor's sound unless a touch picked it (r5).

## Menus outside battle (started)

File select (`trace/s1.state`, the three books): RIGHT moves to the next
book and plays sound 0x11 through 0x0036C438, from 0x00BB9C34 (a UI
element's sound) via 0x00AB9810, called through a vtable from 0x0048CB3C,
which runs a list of commands (vtable +0x20, +0x24). So these menus are
scripted, not the battle UI's code, and 0x0075D420 is not used.

Their touch input is one class, vtable 0x00CFC340, on file select (object
0x1403A380 in s1), chapter select and the Fiend's Cauldron (0x14012690 in
s11 and s12). Found by the functions that run only with a touch held, from
`ran` with and without one: 22 in all three states.

- Its touch listener (secondary vtable, slots from 0x00CFC348): press
  0x00A390EC, release 0x00A38D84, held 0x00A38858. Each turns the touch
  into an event and sends it to the object's vtable +0x24: 1 press
  (0x00A38728), 0x40 held (0x00A38650, every frame), 0x10 drag with its
  movement (0x00A38510), 2 release or flick with its speed (0x00A388F8).
  0x00A38858 also draws the blue ring at the touch (0x002FB790).
- Hit test 0x00A38324(out, object, &xy): walks the layers (+0x1EC the
  first, +0x28 child, +0x2C next), each an array of elements (+0x40, 0x110
  bytes each, +0x20 how many), skips those with +0xC0 & 6 or in another
  group (+0xC8, the object's +0x644), tests the point against the
  element's triangles (0x0025F278, +0x9C) and keeps the frontmost (+0x80).
  Out: layer, index (-1 none), the element in r3 on return. On s1: book I
  0x1406D150, II 0x1406D9D0, III 0x1406E2D8, Back 0x1406E938, How to Play
  0x1406EE00, nothing at 300,20 or on ?HOME.
- The screens act on the press only. With press and release returned at
  once (`code 0xa390ec 0xE12FFF1E`, the same at 0xa38d84), a held touch
  on book II, Chapter 2 or Back does nothing. A press on a book or chapter
  lights it (the second press opens it); a press on Back uses it. So a
  held touch cannot light anything here, unlike the pause screen.
- 0x0071C420 reads the touch each frame (the input singleton, +0x2C..)
  and calls the listener: +0xC press, +0x18 held still, +0x10 moved, +0x14
  release (a tap if under 40 px), +0x1C no touch (0x00A391E4 here, empty).
- A press needs no touch: 0x00A38728(object, &xy) called where 0x00A391E4
  runs lights book II, and a second opens it, with no ring; on chapter
  select it lights Chapter 2 without the list scrolling (a real tap
  scrolls it, on its release). A press on what is already lit uses it:
  one on book I, lit as file select opens, opens the file.
- The event's vtable +0x24 is empty (0x00B6F668); 0x0036C198 posts it as
  a message (0x002F3464's queue), and each screen's own class handles it.
  File select's (0x003025B8, run only on file select) compares the
  element's id with its parts (+0x9C, +0xC8, +0xF4 of its table the
  books) and the picked one, +0x34: the same opens it (0x003374D0),
  another is picked and its book turns (RotObj0000). The lit book's index
  is at 0x1404E9EC in s1 (0, 1, 2). Chapter select's (0x0031164C) only
  stores the flags and position (+0x5B8.., file select's at +0x508) for
  its update and 0x00344400. What a press does, and what is lit, is each
  screen's.
- The element's +0 points at its part's first name (Book01, ML_B0002,
  BL_B0000 for Back, Move Out, the pot and the bar alike) and +4 is 2 or 3
  on chapters: neither tells a picking part from a button.
- Parts are named and known by a hash of the name. The hit test's layer
  +0x38 is the part's object; 0x00326D74(it, &0x00275C18()) +0x1C is its
  descriptor (vtable 0x00CFEB2C): +0x18 the hash, +0x1C the name (a
  string object). Book II's layer leads to 0xA70D5C64 "book_02".
- File select (object 0x1404E918 in s1, vtable 0x00CEC3F4): the part with
  the outline at +0x34 (hash) and +0x38 (name), the book at +0xD4. Its
  update (0x002BBDE8) finds the part's entry in its navigation table
  (0x003023F0: +0xC the part, +0x38 right, +0x64 left, +0x90/+0xBC
  return, +0x140 guide, each a hash and a name; book_01, book_02,
  book_03, return, guide) and moves the outline with 0x00302488(screen,
  &{hash, name}), or LEFT/RIGHT through 0x003381AC(screen, -1/1); A is
  0x003374D0(screen). Called where 0x00A391E4 runs (the harness copy's
  `callf`), 0x00302488 with book_02 outlines book II, the panel follows
  and sound 0x11 plays (0x00BB9C34, 0x0036C438) as for RIGHT, and nothing
  opens; 0x003374D0 then opens it. A descriptor's +0x18 is the {hash,
  name} 0x00302488 takes.
- File Select replay/editor (3 Nov): the 2D foreground-material override
  removes the 3D book covers while leaving their numerals and names. In
  `s1`, the red Book II sample has zero artwork pixels with that override,
  versus 322 after retaining native materials/depth; the native bottom
  sample remains 1,918 pixels. `MenSelfileBt0000/1/2` are Book I/II/III,
  `MenSelfileBt0003` is Back, `MenSelfileBt0004` is How to Play.
  Their hashes are `A70D5C65/A70D5C64/A70D5C67/928BEAA4/8D79D12E`.
  `MenDemoBt0000` is the bottom background; HOME text is a separate font
  draw, not an activatable Control. The complete information Board is
  `MenSelFileUp0001` in layer B.
- The File Select editor indices are 0..2 books, 3 Back, 4 How to Play,
  5 HOME hint, 6 information Board, 7 background. Native book-name font
  anchors remain in their respective thirds of the 320px bottom screen,
  above y190; HOME anchors are around (102,207) and (68,226). Select a
  book's name by that native column, preserving all native shadow draws,
  and let its 3D cover turn outside the nominal Placement source box.
  The native hit test validates those extensions, not a second hit layout.
  Retain the original full-Board viewport/projection and expand only the
  book's scissor. A full-canvas isolated projection displaces the native
  font draws and reveals book faces normally clipped off-screen. The
  corrected capture has zero unexpected pixels to the left of the placed
  Boards, with names attached to their books at normal and wide aspects.
- File Select's black cross-screen connectors are the `RotObj*` mesh
  nodes: `RotObj0000` on Book I and the information Board, `RotObj0004`
  on Book II and `RotObj0005` on Book III. Hide those nodes only in
  transformed File Select replay; retain `Book01/02/03` and `Book04/05`
  cover/page meshes, native materials, and the original bottom pass.
  Suppressing `Book04/05` did not remove the connectors and was reverted.
  The capture strip above the covers changed from 76/51/34 black pixels
  for selected I/II/III to 0/0/0 after suppressing `RotObj*`.
- Editor visibility is stored separately from geometry as hidden
  `(screen, part)` pairs. Hidden entries retain their editor indices and
  source boxes but leave draw order, pointer candidates and explicit
  navigation destination lists. An empty draw order for an active Screen
  must not fall back to its unfiltered parts. File Select HOME is index 5;
  a moved HOME rectangle changes from 4,800 visible pixels to zero when
  hidden, survives a console-state reload, and returns when shown.
- Cardinal File Select seams, after native repeat checks and accessor
  `003023F0`: Left `002BC5A4`, Right `002BC618`, Up `002BC688`, Down
  `002BC6CC`. r0 is the current graph node, r5 the Screen object. A linked
  action calls native `00302488(r5, target_node+0xC)`, which sets both the
  selected hash/name and book index before calling `003381AC(...,0)`.
  Resume at `002BC66C` after Left/Right or `002BC6E8` after Up/Down so
  the remaining native input handling stays intact. A self-link resumes
  without calling the Outline helper. A plain directional-slot bias is
  insufficient: native Left/Right also performs its adjacent book step.
  Live checks cover all four directions to Back, I→III and III→I with
  matching `+0xD4` indices, a held self-link, native-default restoration,
  circle-pad input, and native A opening How to Play.
- Chapter select (object 0x1403E7A0 in s11, vtable 0x00CF243C): the part
  with the outline at +0x6C/+0x70 (BtThumbnail on the chapters; DOWN
  moves it to Return through 0x003668E8, which posts it). The chapter is
  +0x12C (and +0x130), the last one open +0x128. 0x00343FF8(screen,
  chapter, 1) picks one as LEFT/RIGHT do (0x003441F8): called with 1 it
  outlines Chapter 2 and scrolls the list, but without RIGHT's sound 7,
  which comes another way.
- So the outline is each screen's own: a hash and name at its own offset,
  and its own function to move it and to use what has it. The part under
  the mouse is the same for all of them.
- The hit test's out: +0 the layer, +4 the element's index (-1 none), +0xC
  the element. The task loop runs a menu's update through a thunk, the
  object in r0: file select's 0x002A3EC4 (to 0x002BBDE8), chapter
  select's 0x002A3FCC (to 0x002D7404, then 0x00310880 for its keys).
- Chapter select's list is one layer (0x140511D8 in s11) of 33 elements:
  MenStageselectBt, ML_B0000, ML_B0001 to ML_B0026 (each chapter, named
  by its number; ML_B0009 and ML_B0023 also _0 and _1), NN_B0000. The
  chapters not open have +0xC0 0x2002 and are not found. The index stays
  with the chapter as the list scrolls (Chapter 1 is 2, Chapter 2 is 3).
  The element's +0 is its name here, so the chapter is the number less 1.
- Chapter select: 0x00367394(object) outlines the list (BtThumbnail) as
  UP does from Back, 0x003668E8(object) Back (Return), each posting it;
  0x00343FF8(object, chapter, 1) scrolls to the chapter and sound 7 plays
  as for RIGHT. 0x003675C0(object) is A: on the list, the chapter's
  Fiend's Cauldron (0x0038B214), on Back, back (0x003672BC); from s11 it
  goes on to the cauldron the same frames as A.

- Main menu (`trace/s14.state`, your 1.state of 30 Oct): object
  0x1403D640, vtable 0x00CFBD9C, updated through 0x002A3F9C (to
  0x002EBA60, which redraws the outline when +0x9C is set, then
  0x0034E28C). Its keys, 0x003201D8(object, input): directions step a
  table by 0x0034E1B8(object, direction; 5 RIGHT), which writes the
  outlined part's hash at +0x44 and its name (a string) at +0x48, copies
  them to +0x70/+0x74, calls 0x0036FB58(object, 0) (reads only +0x44 and
  +0x48, sets +0x9C) and plays the cursor's sound, 0x002B435C(+0x30, 7).
  A (input 0xF500) is 0x0036F59C(object); B 0x200 goes back. The hit
  test's parts carry the same hashes: Solo 2E49363F, StreetPass 5407F273
  ("Present"), Vault E250C577, Together E2DCC735, Options D929D345, Back
  9A07E025, AR 4A7AE3E3, guide FD90A21D. Functions of the 0x0034E/0x0036F
  family may be shared by the hub's other button menus.
  The shared directional-name resolver is `0x003506C4(record, direction)`.
  Cardinal codes observed with real pad input are Up 7, Down 1, Left 3,
  Right 5. A graph record contains its own hash/name at +0xC; linked records
  are reached through +4. Direction 9 takes the default/self-name path.
  Editor links redirect this resolver to the destination record with direction
  9, leaving native repeat, availability checks, name copying, sound and A
  activation in the original caller. The s14 replay confirms four cardinal
  overrides, circle-pad Right, a held self-link, and native fallback after
  removing a link. Diagonal codes are not overridden.
  Solo Hub inlines the directional record reads after `0x0034E870` returns:
  `0x00321720` Left (+0x64), `0x0032178C` Right (+0x38),
  `0x003217F8` Up (+0x90), `0x00321864` Down (+0xBC).
  These hooks bias the returned record so the caller reads the destination's
  self-name through its existing slot. Save-state checks redirect Move Out
  Right to Practice Range and Arms Altar Redeem Hearts Right to Back.

- Solo hub (`trace/solo.state`, from s14's Solo): object 0x1403A5E0,
  vtable 0x00CFC3D0. Its live update is 0x002ECCF4 (called directly by
  the task dispatcher, rather than through thunk 0x002A3EF8). The current
  hash/name is +0x5C/+0x60, pending +0xFC/+0x100. Directions in
  0x00320E7C call 0x002BFE50(object+0x4C, part), queue the pending name,
  and set +0x150 bit 0. The update posts its notification, copies pending
  to current, and moves the camera by 0x00321D28. A is 0x00370300(object),
  also used by the touch handler 0x0034E944. +0x144 == 1 is the opening
  guide, which owns input. Hit-test hashes: Move Out 79BCE02D, Arms Altar
  38C8C232, Practice Range 94CE5E14, Idol Toss 4F73A4C0, Gear 71966FCC,
  Treasure Hunt ED263933, Back 9A07E025.
  A click that needs to change the outline must let the update consume
  its queue before calling A: otherwise clicking Arms Altar after DOWN
  moved to Gear opens Gear. Shared menu call handling now waits for the
  next update after its final outlining call before using the choice.

- Arms Altar's button menu (`trace/altar.state`, from Solo's Arms Altar):
  object 0x1403E7C8, vtable 0x00CFCA28, input 0x002A1FD8 (called by
  active update 0x0025F940 and transition update 0x0025FAEC). Current
  hash/name +0x44/+0x48. Directions go through 0x002EF084 and touches
  through 0x002EF154; both call 0x00322DB0(object, &name, repeat), which
  moves the outline, posts its notifications, updates the animations and
  plays the cursor sound. A is 0x00322C34, Back 0x0032287C. Hit-test
  hashes: Redeem Hearts 1B80430F, Fuse Weapons C59BD435, Convert to Hearts
  E43D88E9, Gear 71966FCC, Back 9A07E025. The mouse calls 0x00322DB0 with
  repeat 0, then A after a click. RIGHT from Redeem Hearts selects Fuse
  Weapons; clicking with the pointer still over Redeem Hearts correctly
  returns there and opens the shop. Harness: `trace/menualtar.txt` and
  `trace/menualtar-{fuse,convert,gear,back}.txt`.

- Mouse gestures: the frontend no longer consumes a mapped menu button
  on mouse-down. `AimMouse::menu_touch` waits for release to call A, or
  passes a held native touch after 12 emulated frames or four pixels of
  movement. It begins at the original point for three frames, then
  follows the pointer, so fast movements still grab the original item
  through a 30 FPS input poll. Unmapped controls pass native touch
  immediately. `outline_menu` leaves native touch and its release alone;
  losing focus cancels the pending click. The frontend polls held input
  even without cursor events. Harness `press <frames>` / `release` uses
  the same routing; power-board pickup, drag and release match native
  touch on both screens (`trace/powerdrag-{mouse,native}.txt`).

Armory mouse support uses seven native input updates: Gear 0x002B6DB8,
Redeem 0x00238CBC, Convert 0x001C9734, Fuse 0x00230B5C, Powers
0x002BB14C, weapon list 0x0019CDC8, categories 0x0019BDC4. Their A/B
event records preserve native validation and confirmations. Generic lists
keep absolute selection at +0xE4, first visible row at +0xEC, row height
at +0xFC and animated offset at +0x11C; 0x0031AA88 scrolls their viewport
and 0x0031AED8 refreshes the cursor. Fusion uses independent horizontal
and vertical offsets +0x2B28/+0x2B30, bounded by count +0x2B40 minus
seven visible items. Category navigation uses native direction tables and
skips unavailable categories. Part-component tree lookup terminates at
its head sentinel; +0x10 is node color, not a leaf marker.

- Options and its submenus (states s15 Options, s16 Other, s17
  Controls, s18 Volume, s19 Profile, s20 Subtitles, s21 Reticle/Camera,
  s22 Control Configuration, s23 More Controls, s24 Music Gallery, s25
  Profile's Yes/No, s26 Air Battle's settings, s27 SpotPass's No/Yes; all
  made headless from s14). Each is its own class, most a copy of one
  pattern: the outlined part at +0x44 (hash) and +0x48 (name string); a
  key queues the next one and an early step of the update (a copy of
  0x002B4428) outlines it, posts it to the buttons' animations and plays
  the sound from a scripted element (0x00BB9C34). Per screen (vtable,
  update thunk, how a key queues, A):
  - Options 0x00CEAE98, 0x00224BA0: 0x002BFE50(+0x34, part), then
    0x002EA7A0(+0x9C, part), +0xD4 |= 1 (bit 1 back); A 0x002FB9E0.
  - Reticle/Camera 0x00CF9B60, 0x001B4C74: as Options; A 0x0029B26C.
  - Other 0x00CF2368, 0x001ECB4C: one function, 0x002D5430(object, part,
    direction), which also writes +0xC8/+0xCC 2 for SpotPass. Left 0
    (queueing by hand), SpotPass's switch (0x001CFF5C) takes the move for
    a touch on it and sets the touch object's group, +0x654, to its own
    (0x12): the hit test then skips every other layer, and no touch (real
    or the mouse's) finds Profile. A 0x002D50FC.
  - Controls and More Controls 0x00CF84DC, 0x00224C18: 0x002EA7A0(+0x9C),
    +0x118 = 1, +0x108 = -1, +0x11D bit 3 direction, then
    0x00318DD0(object, part) for the rows' animations; outlined by
    0x002E1CF8. A 0x003186E8 (toggles a More Controls row).
  - Volume 0x00CF2390, 0x00224BD4: 0x002D5B40 queues at +0x9C, zeroes
    +0xE4/+0xE8, +0xD4 |= 1; outlined by 0x002D562C. A 0x002D56CC.
  - Profile 0x00CF8E8C, 0x001ECBF4: hash +0xA4, name +0xA8, +0xDC bit 0;
    outlined by 0x002952F4. A 0x002E2F4C.
  - Profile's Yes/No 0x00CEC41C, 0x0016756C: hash +0x9C, name +0xA0, +0xF0
    bit 0; outlined in 0x00172A50. A 0x001C16D4.
  - Subtitles 0x00CFA034, 0x001ECBBC: the outlined one an index, byte
    +0x60, into parts at +0x18 (0x2C each) of what +0x30 keeps of kind
    0x00D70E54 (the map 0x00326D74 walks: a sorted tree at +0x1A8/+0x1AC);
    0x002E5608(object, part, direction) outlines one. A 0x002E5448.
  - The hub's choice box (SpotPass's No/Yes) 0x00CF8270, 0x002A4070: one
    part (BtDialog), each button an element fnt_02, fnt_03 (index 0, 1 at
    +0x44); keys set the index then 0x0028F8EC and 0x002DC68C light it;
    0x00314DF8(object) uses it (A and a touch).
  - Music Gallery: a scrolled list, its cursor moved by 0x002576C4(object,
    step) (0x0021E7C0 its keys); a tap picks a row. Control Configuration
    and Air/Land Battle's settings: no outline posted on keys; touch
    screens (sliders, a test pad).
- Two of these update at once while a choice box is over its menu (Other
  under SpotPass's box), so the mouse goes to the newer of the two.
- A save state made on Windows keeps each open archive's Windows path;
  loaded here the game could not open gamecoin.dat (shared extdata) when
  Options opened and stopped (0xC8804471). Loading now moves each archive
  under this run's user directory.

- Practice Range (`trace/practice.state`, from Solo's Practice Range):
  object 0x1403E7C8, vtable 0x00CF0154, its update 0x0023A9FC (through its
  thunks 0x00224C50, 0x00224C54, 0x00224C58). The outlined part a hash at
  +0x2E4 with its name after it (+0x2E8, a string whose buffer is at +0x20).
  A key writes the next one's name into a queue at +0x33C and sets +0x374's
  bit 0; the update's step 0x00283DE0(object) copies it in with
  0x002EA7A0(+0x2E4, +0x33C), which computes the hash, plays the cursor's
  sound and marks the part. A is 0x0030B5B4(object): Air Battle and Land
  Battle start their practice with 0x00340FF4 and 0x00340ED4(object, 0 or
  1), Gear opens the armory with 0x0030B68C(object), Back (the return hash)
  goes back. The parts: AirTraining 0xE50FA312, GroundTraining 0xADD1FD65,
  Equipment 0x71966FCC, Return 0x9A07E025. On the bottom screen Air Battle
  is about (160, 50), Land Battle (160, 150), Gear (160, 215), Back (20,
  200 to 230); the left arrow and the top screens' titles and hints are the
  game's own for the key.
- Idol Toss (`trace/idols.state`): object 0x14040508, vtable 0x00CF9400,
  its update 0x001B4D58, a thunk of 0x001E05D4, which reads the pad into
  +0x2E0 and +0x2E4 and calls 0x00217A0C(object). The selected part an
  index at +0x3B8 into the scene's parts at +0x44, +0x254 and +0x1D0 (the
  scene is what its task, 0x1403D258, keeps of kind 0x00D71458), a hash at
  +0x48 and its name after it. Its keys step the index with
  0x0024E9F8(object, input) and copy both; a touch, 0x001E0B60(object),
  calls 0x0022CF38(object + 0x38, +0x48), which finds the part under it.
  The bowl's hash is 0x4F14908C, Back is 0x9A07E025. These parts act on
  touches and not on A: the bowl does nothing at this save's point, Back
  goes back on release.
- Treasure Hunt (`trace/treasure.state`): object 0x14040508 as well,
  vtable 0x00CEEDD8, its update 0x0018AF44, a thunk of 0x001A0C64, the
  pad's bits into its elements' flags; 0x001A0AE4 its other update. What
  is picked a hash at +0x34 (Back 0x3B8915AB) and its name after it;
  0x0020207C(object, part) sets both from a part, as its touch handler
  0x001CBFCC does. The board's tiles are not parts the hit test finds, so
  the mouse outlines none of them: its press touches where it is, and the
  tile under it is picked, its idol shown on the top screen.
- Vault (`trace/vault.state`, from the main menu's Vault): object
  0x1403A5E0, vtable 0x00CF8248, its update 0x00224B80, a thunk of
  0x00240164, which copies the pad into +0x2D8..+0x2E8 and then runs
  0x0028E6AC (the input's bits into the parts' flags at +0x194, +0x1B8
  and on, 0x24 apart) and 0x0028E5E4 (the hash at +0x44 to a part index,
  0..7, and its scene's data to +0x70's +0xC). The outlined part its hash
  at +0x44 and its name after it; 0x00345C18(object, part, 1) sets both
  from a part, as the class's pointer handler does (0 for a key), and A is
  0x00345FDC(object), which presses whatever hash +0x44 holds. The parts:
  Idols E1D05CA4, Music Gallery 0A9FA648, Power Portrait ABC93BCF,
  Offering 04E786EA, Idol Toss 84D28812, Records 9EC09AD0, AR BEA7E6CA,
  Back 9A07E025; on the bottom screen Idols is about (100, 60), Music
  Gallery (250, 60), Power Portrait (60, 145), Offering (160, 145), Idol
  Toss (240, 145), Records (160, 195), AR (285, 225), Back (40, 225).
  Vault's direction handler calls the shared resolver `0x003506C4` (return
  address `0x002DC4F4` in `f_002DC458`) with the current part's hash at
  record `+0xC`, so editor links redirect it exactly as the hubs do. Verified
  from `trace/vault.state`: Right from Idols with a link reaches Back
  (9A07E025), and removing the link restores Records (9EC09AD0).
- Options and its Scene child pages: Options' four parts are Options
  6C7DCDF1, Volume 53977FC2, Other FB57B10C and Back `928BEAA4` — Back is
  *not* the shared 9A07E025. Practice Range's are Air Battle E50FA312,
  Land Battle ADD1FD65, Gear 71966FCC and Back 9A07E025. Their key handlers
  inline the graph lookup rather than calling `0x003506C4`. Options gets
  its current node from `0x002B481C`, returning to Left `0x0026E0A0`,
  Right `0x0026E114`, Up `0x0026E18C`, Down `0x0026E204`. Practice gets
  it from `0x002CF954`, returning to `0x00284898`, `0x00284948`,
  `0x002849F8`, `0x00284AA8` respectively. Their name slots are +0x64,
  +0x38, +0x90, +0xBC, as Solo's. All eight cardinal redirects to Back
  passed native Cursor assertions. The earlier `0x003D4114` observation
  concerned graph rebuilding, not directional input; compiled callees
  were invisible to the old `calls` probe. `calls` now interprets its
  requested targets before stepping.
- The Options child Screens use the same circular navigation nodes
  (current hash +0xC, next +4) despite different current-node accessors.
  The editor hooks these accessor returns before the caller reads a name:

  | Screen | Left | Right | Up | Down |
  | --- | --- | --- | --- | --- |
  | Other | 0023C1AC | 0023C238 | 0023C2C8 | 0023C384 |
  | Controls/More Controls | 002E135C | 002E1854 | 00293848 | 002938CC |
  | Reticle/Camera | 00219A64 | 00219AD8 | 00219B50 | 00219BC8 |
  | Profile | 002474B0 | 0024751C | 00247588 | 002475F4 |
  | Profile Yes/No | 0019560C | 0019568C | 0019570C | 00195784 |
  | Subtitles | 002535C0 | 00253610 | 00253660 | 002536B0 |

  Their current-node accessors are 00289880, 00319228, 00252168,
  002958E8, 001C1DC0 and 0029B8BC respectively. All cardinal redirects
  passed native Cursor checks, including Subtitles' byte +0x60 (Off = 0).
- Armory — Gear (vtable 00CEC2B8) uses the same graph through its accessor
  `0x002FCD34`, from key handler `f_002B6DB8`, outlining with
  `0x00334D3C(object, name)` and keeping the hash at +0x2E4. The editor
  hooks its returns: Left `002B75E4` (+0x64), Right `002B7638` (+0x38), Up
  `002B7688` (+0x90), Down `002B76E0` (+0xBC). Parts: `MenEquipBt0001`
  Weapons/A754BFE8, 0002 Powers/CE77C7CF, 9999 Back/9A07E025, 0000 the
  backdrop. Verified from `trace/armory-gear.state`: Down reaches Powers
  natively, Back when linked, and Back's linked Right reaches Powers.
- Armory — Categories (00CEE1C0) steps an index, not names: tile at +0xDC
  (0 All to 9; 10 Back), previous at +0xE0. Its key handler (from `0x0019C470`) calls
  `0x001C66BC(object, table)` per direction, Left 00CA3180, Right 00CA2AA0,
  Up 00CA2730, Down 00CA2E10 (four more are diagonals); each table has 11
  rows of 0x28 bytes listing candidates, -1 terminated, the first with
  weapons (`0x001FC6C4(tile - 1) > 0`) taken at `0x001C670C` from r4. From
  Back the step returns to +0xE0 whatever the candidate. The editor hooks
  the loop head `0x001C66CC` (r6 = 0) to take a linked tile, writing +0xE0
  first when leaving Back; `0x001C66E4` returns 0 for a self-link. Tiles
  are `MenEquipBt0102`..`0111` in index order, Back `MenEquipBt9999`.
  Verified from `trace/armory-weapons.state`: All→Back, Back→Bow and Bow's
  self-link, with native Down from Bow reaching Club (6).
- Armory model roles: Weapons list `MenEquipBt0120` rows (0125 cursor),
  0121 order (X), 0122 page bar, 0123 Equipped (Y), 0100/0101 backdrop.
  Powers `MenEquipBt0203` grid and tabs (0206 pieces), 0211 list (0215
  cursor), 0204 Autofill, 0212 category header, 0205 OK, 0200 backdrop.
  Replaying the whole Board for each Control stops the top replay after a
  few frames (drawing buffer); each Control draws its own models, and only
  Controls that show text replay the text pass.
- Harvested child Control identities from native hits:
  Other: `MenOptionOtherBt0001` name/49C14849, 0002 caption/F28BAA60,
  0004 itunomani/9E610C53, 9999 return/928BEAA4.
  Controls: `MenOptionControlBt0001` button/DE4F96DF,
  0002 touch/56866028, 0006 opt/FB565F88, 9999 return/928BEAA4.
  More Controls: 0004 guide/8D79D12E, 0008 lead/4998C4E7,
  0007 asis/4A0C9579, 0003 auto/4A0F894B, 0005 test/49B8FADA.
  Its mode is bit 0 of object +0x11D: clear for Controls, set for More
  Controls (confirmed in states s17/s23 and 002E19F0..002E19F8).
  Reticle/Camera: `MenOptionCtlTpenBt0001` groundBtn/E84BA0DD,
  0002 airBtn/21E46FF0, 9999 return/928BEAA4.
  Profile: `MenOptionNameBt0001` Name/4A41C1D7, 9999 Return/9A07E025.
  Profile Yes/No: `MenSelnameBt0001` BtYes/3837AABD,
  0002 BtNo/4F14658D, 9999 BtReturn/3B8915AB.
  Subtitles: `MenOptionOtherCaptionBt0001` upperButton/6A1911BF,
  0002 lowerButton/E76D363A, 0003 offButton/DD5F933C.
- Volume: `MenOptionSoundBt0002` music/6AFB5539,
  0003 se/CD949985, 0004 voice/002A0EEA,
  0005 soundTest/1C05D789, 9999 return/928BEAA4.
  Models 0001 and 0006 contain shared labels/slider pieces; keep them
  with each slider, clipped by its Placement. Native directional helpers
  002D590C/002D5B40/002D5D74/002D699C handle Up/Down/Left/Right.
  Linked directions queue the chosen name from retained kind 00D70ED8,
  using the native key-repeat flags and 002EA7A0; unlinked directions
  retain the original slider adjustment. In s18 Music's float at +0xD8
  (1403EA08) is 79: a moved slider drag changed it to 10; holding a Left
  self-link for 40 frames left it at 79.
- SpotPass Dialog: native object 1404F4E0 in s27, choice at +0x44
  (1404F524): No = 0, Yes = 1. Both hit BtDialog/B1EBC922; distinct
  elements, not distinct model hashes. Split its caption and choices by
  native source rectangles and crop the shared MenCmnWindowBt0000 model.
  The store at 00241368 keeps native repeat/lighting/activation; LR
  002412A4 identifies Left and 0024130C Right. Native Up/Down do not
  select a choice, so the editor reports those axes unavailable.
  Both horizontal self-links passed Cursor assertions; a moved No at
  native (80,190) dismissed the Dialog without enabling SpotPass.
- The Vault's Idol Toss is the hub's screen again, its object at 0x14041518
  here and its update 0x001B4D4C, not the hub's 0x001B4D58: the same class
  dispatches by event, so both thunks are in MENU_SCREENS. Its Back is the
  whole strip below y 200.
- Its other screens are chrome over a viewer: the Idols viewer (object
  0x14040508 in its state, vtable 0x00CF93D8, update 0x001B4D70), the
  Music Gallery's list (object 0x1403E938, vtable 0x00CFB010, updates
  0x001B4D28 and 0x001B4CF8), Power Portrait, Records and AR. They have no
  outline state of their own: a tap selects (Music Gallery's rows and its
  play mark), a tap on Back leaves, and the pointer's press is that tap.
  Their `+0x44` is not a part's hash (the Idols viewer's is a constant
  0x928BEAA4) and 0x00345C18 does nothing on them.
- The Music Gallery's list is the screen's own, at +0x190 (0x1403EAC8 in
  that state): the track it has picked an index at +0xE4, the first row it
  shows at +0xEC, its rows at +0xD0 and the ones it shows at +0xC8. Its
  keys go through 0x0031AA88(list, down, move): with the third word 0 it
  moves the first row by one (0x002E5B80(list, rows)), with 1 it moves the
  picked track, down = 1 up = 0 - called by hand it took the pick 0 to 5
  and back - but the rows it shows did not come back with it and the probe
  left fewer of them showing, so the third and fourth words the keys pass
  are not known yet and the wheel is not wired to it.

- Screens whose parts act on touches, not on A (Idol Toss, Treasure Hunt),
  take no click of their own: `CLICK_TOUCHES` in their MENU_SCREENS entry
  keeps them out of `menu_clickable`, so a press over a part becomes the
  touch at the pointer, as for unmapped controls.

## Screen anchors

Canonical Screen names are in `GLOSSARY.md`; this table is the only place their code anchors live. Scripted Menu rows come from `MENU_SCREENS` in `tools/zakuro/crates/zakuro-core/src/aim_mouse.rs`; `vtable` is the object's first word.

| Screen | vtable | update | outline | A / use |
| --- | --- | --- | --- | --- |
| File Select | 0x00CEC3F4 | 0x002A3EC4 | outline_file | 0x003374D0 |
| Chapter Select | 0x00CF243C | 0x002A3FCC | outline_chapter | 0x003675C0 |
| Main Menu | 0x00CFBD9C | 0x002A3F9C | outline_buttons | 0x0036F59C |
| Options | 0x00CEAE98 | 0x00224BA0 | outline_queued | 0x002FB9E0 |
| Other | 0x00CF2368 | 0x001ECB4C | outline_other | 0x002D50FC |
| Controls | 0x00CF84DC | 0x00224C18 | outline_controls | 0x003186E8 |
| Reticle/Camera | 0x00CF9B60 | 0x001B4C74 | outline_queued | 0x0029B26C |
| Subtitles | 0x00CFA034 | 0x001ECBBC | outline_subtitles | 0x002E5448 |
| Volume | 0x00CF2390 | 0x00224BD4 | outline_volume | 0x002D56CC |
| Profile | 0x00CF8E8C | 0x001ECBF4 | outline_profile | 0x002E2F4C |
| Profile Yes/No | 0x00CEC41C | 0x0016756C | outline_yes_no | 0x001C16D4 |
| SpotPass Dialog | 0x00CF8270 | 0x002A4070 | outline_dialog | 0x00314DF8 |
| Solo Hub | 0x00CFC3D0 | 0x002ECCF4 | outline_solo | 0x00370300 |
| Arms Altar | 0x00CFCA28 | 0x002A1FD8 | outline_altar | 0x00322C34 |
| Armory — Gear | 0x00CEC2B8 | 0x002B6DB8 | outline_armory | none (list input) |
| Armory — Redeem Hearts | 0x00CEFCA8 | 0x00238CBC | outline_armory | none (list input) |
| Armory — Convert to Hearts | 0x00CEEBFC | 0x001C9734 | outline_armory | none (list input) |
| Armory — Fuse Weapons | 0x00CEE148 | 0x00230B5C | outline_armory | none (list input) |
| Armory — Powers | 0x00CEC2E0 | 0x002BB14C | outline_armory | none (list input) |
| Armory — Weapons | 0x00CEE1E8 | 0x0019CDC8 | outline_armory | none (list input) |
| Armory — Categories | 0x00CEE1C0 | 0x0019BDC4 | outline_armory | none (list input) |
| Practice Range | 0x00CF0154 | 0x0023A9FC | outline_practice | 0x0030B5B4 |
| Idol Toss (Solo Hub) | 0x00CF9400 | 0x001B4D58 | outline_idol | none (click is a touch) |
| Treasure Hunt | 0x00CEEDD8 | 0x0018AF44 | outline_treasure | none (click is a touch) |
| Vault | 0x00CF8248 | 0x00224B80 | outline_vault | 0x00345FDC |
| Idol Toss (Vault) | 0x00CF9400 | 0x001B4D4C | outline_idol | none (click is a touch) |
| Control Configuration | in-game Options page 5 | 0x00A0ACF0 | none | drags what it holds |
| Pause | object at ctx+0x243C | 0x0060DC7C input | pause options page at +0x678 | touch: 0x00A0867C main page, 0x00A06188 Options |
| Retreat Dialog | dialog object | 0x007226C4 input | picked button at +0x294 | touch began now |
| Music Gallery | 0x00CFB010 (list) | 0x001B4D28, 0x001B4CF8 | none | keys 0x0031AA88 |
| Fiend's Cauldron | pot task 0x00CF9AEC | 0x00251624, 0x00251C40 | cursor class 0x00CEE170 | touch |

## The Fiend's Cauldron

States: `trace/s10.state` (your 9.state of 29 Oct, the bar), `s11` chapter
select just before it, `s12` the bar, `s13` just after Move Out (your 1-3
of 29 Oct, 19:06).

- Its 2D is not the battle UI's: nothing goes through the sprite batch
  flush (0x0092F408) or a 2D pass start (0x0092E4F8) but the dialog
  context's corner icon. It is drawn by the scripted layout system.
- Bottom screen boxes, from a screenshot (bottom pixels): "Try This?" tag
  y 0..12 above the bar, bar x 22..322 y 13..30, the skull dial y 40..80
  sliding along the bar with the value, pan and pot x 70..250 y 75..190,
  the bet counter x 218..310 y 143..170, Back x 10..45 y 205..238, Move Out
  x 60..250 y 185..240. The top screen's marker stem meets the bar at the
  same value 40 px to the right (top x = bottom x + 40), as for the pause
  menu.
- 0x0067E468 (vtable 0x00D00248, +0x10) binds a parameter table of the
  screen's parts by name (startBody_ "GO" = Move Out, returnBody_ "return"
  = Back, level_bar_ "Level_Bar", saturn_pot_ "Saturn_Pot", go_uranai_,
  saturn_dish_, ...): an object at 0x14B91158, nothing calls into it per
  frame. Animation entries naming the parts (vtables 0x00CFC4DC,
  0x00CEC7D0) sit at 0x14914668.. with an owner 0x14050A50, also not used
  per frame.
- Per frame, the screen runs as tasks from the task loop (lr 0x00417680).
  Only here, against states 1-9 (`ran` in the harness): tasks with vtables
  0x00CEE170, 0x00CF7344, 0x00CF9310, 0x00CF9338, 0x00CF9AEC, 0x00CF9FE4,
  0x00CFA00C, 0x00CFA330 (objects 0x14051270, 0x1406A950, 0x1406C358,
  0x14063858, 0x14045F68, 0x14051C30, 0x1404B0A8, 0x140554D8). Which of
  them are the cauldron's and which any menu between chapters has is
  still to tell, with a state on another menu (chapter select).
- Telling it's shown (the fork's `AimMouse::cauldron_shown`): the pot's
  task (vtable 0x00CF9AEC, 0x14045DB0 in s12) runs 0x00251624 every frame
  while the bar is up, and 0x00251C40 while it pours after Move Out (its
  update pointer moves on); neither runs on chapter select, after the
  pour (a black fade, then the battle loads) or in battle (states 1-9,
  11). Checked with the harness's `shown`: on in 10, 12, 13, off in 11, 4
  and 5, on for about 90 frames of the pour from 13, then off.
- While pouring, the pan lifts to the top of the bottom screen (y 10..70)
  and the bet counter with it; the bar is gone and the top screen fades.
- The loop at 0x00A35750..0x00A35C1C, which draws through the 3D renderer
  (0x00F2DC9C) with a projection for the top (0x00DB72D0, once for each
  eye at x +-18.3) or the bottom (0x00DB7290), draws only the text lines
  ("fnt_%02d"): with its draws (0x00909ACC at 0x00A35B00, 0x00A35BC8)
  skipped, only the hint line is gone. Everything else is models in the
  scene, below.
- Tried and dropped: composing the window from crops of the two screens'
  pictures in the presenter. It works but looks pasted together (cut
  edges, boxes of the bottom screen's background).

How the screens are drawn (all from `trace/s12.state`):

- A frame renders render passes (vtable 0x00D16058, run by 0x00BA9E14):
  "PreRenderPass", "TopRenderPass", "BottomRenderPass", each a list of
  drawings at +0x40, +0x50 how many, the screen at +0x58 (-1, 0 top, 1
  bottom), started by 0x009167C0 (0x00916588 picks the buffer: 0 the left
  eye, 1 the bottom, 2 the right eye). Top and bottom draw the same scene
  (0x149A6378) with different cameras.
- The scene: cameras by screen from +0xC8 (0x00E14BEC top, 0x00E14DCC
  bottom, both perspective lenses), layers 0x28 bytes each from +0xEC,
  each with a list (+4) of models at +0x18..+0x1C. A model (vtable
  0x00D14AF4) has its CGFX model (CMDL) at +8, its runtime meshes after
  it (0x88 each), and its mesh nodes' visibility at +0x17C (8 bytes each,
  shown at +4), in the CMDL's +0xD0 dictionary's order. 0x008F5FB4(model,
  mesh) is the visibility test; the CMDL's own flags are not read.
- The top's drawing (vtable 0x00CEF1A0, 0x0072A034): 0x00727804 sets the
  viewport from +0x44 (x, y, w, h, the buffer turned: x and y count from
  the screen's right and bottom) and the camera, then 0x00727C24 draws
  layers 0xB..0xE, each the right eye, the left eye, then its text
  (0x00718120 at 0x00727F10; the hint is layer 0xB's). Layer 0xB: the
  picture, one model `MenBetheartUp0000` (nodes: polySurface45 the dark
  frame around the board and the hearts' box, polySurface51 the shading,
  polySurface47 the intensity panel), the hearts' number and icon.
- The bottom's drawing (vtable 0x00CF6D6C, 0x007C8ECC): the viewport, the
  bottom camera, layers 0xF..0x12 when +0x5D is set (loop ends at
  0x007C91C4), then the bottom 2D. Layer 0xF: `MenBetHeartBt0003` pot,
  `Bt0004` pan and hearts bet, `Bt0002` bar and dial, `Bt0005` "Try
  This?", `Bt0006` crown, `Bt9999` Back, `Bt0010` the ring background,
  `Bt0007` glow, two particle models; layer 0x12: `Bt0000` Move Out
  (nodes `button`, `cursor`, the red outline).
- A camera's projection is kept at +0x6C while bit 2 of +8 is set: its
  first row gives the view up the screen, the second across it from the
  right. Scaling and shifting them shows a part of the screen in the
  viewport. The top's stereo camera (0x00916234) rebuilds each eye from
  it and swaps where the shifts land (see `cropped_projection`).
- Mirroring a model by the projection turns its triangles' winding, and
  the materials' face culling (GPU register 0x40, `00010040` after the
  value in the CMDL's commands) then drops them.
- The pot task's +0x6C is the intensity (float). The dial on the bottom
  screen is at x 33 + 28.3 per 1; the top's panel slides under the board,
  its middle at x 64 + 30 per 1.
- The cursor (what the arrow keys light): a task, vtable 0x00CEE170,
  updated by 0x0027B100. It names what is lit at +0xD8 (hash, string):
  `Level_Bar` 0x53D3F0F1, `GO` 0xCD94A288, `return` 0x928BEAA4. Up and
  down go between the dial and Move Out, Back to the dial; left and right
  between Move Out and Back, and move the dial when it is lit. A key
  lights another with 0x003062F0(cursor, name), 0x00305D44(cursor, 1),
  then +0x191 |= 0x10; done before the key handling (at the update's
  entry) the old one stays lit, done at 0x0027BBA0 (after it) it works.
- Move Out acts on the press, not the release, and grows when lit.
- Your cauldron states were saved with the 16:9 widening in both lenses
  (aspect 1.7778 and 1.4222, the game's are 1.6667 and 1.3333): harness
  scripts write the game's back after loading.

## Weapon effects: geometry shader mode 2

Reported with missing Sagittarius Bow and orbital effects and repeated
`geometry shader mode 2 is not implemented` warnings. In Zakuro's
`zakuro-gpu/src/raster.rs`, `geometry_stage` returned no vertices for every
nonzero mode, dropping those draws entirely.

Mode 2 is fixed-primitive input through float uniforms. Register 0x252
holds the mode in bits 0–7, vertex count minus one in bits 8–11, attribute
stride minus one in bits 12–15, and starting uniform index in bits 16–23.
Each complete group of vertex shader outputs fills that uniform range
and invokes the geometry shader. This differs from mode 0, which fills
input registers. Semantics cross-checked against Azahar's
[pipeline registers](https://github.com/azahar-emu/azahar/blob/master/src/video_core/pica/regs_pipeline.h)
and [geometry pipeline](https://github.com/azahar-emu/azahar/blob/master/src/video_core/pica/geometry_pipeline.cpp).

Implemented that path with bounds/stride validation and b15 set after each
invocation within the draw. A rendering regression sends three vertices
with two attributes each to c4–c9, twice with different colors, and checks
the final framebuffer. Both that test and the existing point-mode test
pass. The full Vulkan GPU suite passed 178 tests, with 2 existing benchmarks
ignored. Actual Sagittarius Bow/orbital effects still need an in-game check.

Windows release rebuilt as `dist/zakuro-kiu-mouse.exe`, SHA-256
`dcaa63994cfec8b4b4c9f02e4f1aba02b17d2a57065a0705003d418eedeaea09`.
The previous build is `dist/zakuro-kiu-mouse-before-geometry.exe`.

## Scripted menus on the top screen

The top drawing's common entry after its profiling scope is `0x0072A050`;
`0x0072A0B4` joins its normal and special scene paths. Hooking only
`0x0072A09C` misses subtitles and several other pages. `aim_mouse/menu_top.rs`
uses that common pair to place the native top information, then replay the
bottom's model groups. Original model order determines overlay order, so a
selected hub button keeps its native foreground position. Lists and boards
use one transform, shared with the pointer inverse. Inverse coordinates snap
within 0.0001 of an integer before touch conversion, avoiding a row changing
from 84 to 83 through floating-point roundoff.

During menu entrance animations, hub models can appear before the menu's
input-update hook starts. The renderer recognizes the Solo, main, Arms Altar,
Practice Range, Vault and Options hub models during that interval and uses
their normal layout; input remains disabled until the menu update identifies
its owner. In the main-to-Solo trace, Solo models first appeared 21 frames
before its input update.

The existing bottom drawing stop at `0x007C91C4` avoids replaying native
bottom 2D/scenery. Individual hub controls use model-name allowlists; their
labels are baked into their meshes. Dense pages keep native text and controls
together. The text scissor call at `0x009164A8` and return at `0x00916580`
map clipping through the same transform. Font calls in `0x00A36E64`'s
function temporarily shrink the Cauldron's hint font and restore it afterward.

PICA registers `0x65..0x67` provide include scissoring. The GPU previously
ignored it; include rectangles now intersect the existing raster bounds for
both software and Vulkan. A shifted/oversized viewport must not enlarge the
framebuffer's tile stride: framebuffer dimensions remain authoritative.
Regression tests cover clip color/depth writes and offset viewport rendering.

Drawing dense bottom lists twice with the 3D slider off overflowed the game's
command allocation. Only the left-eye replay is needed in mono. Stereo still
needs a safe command-buffer expansion, so scripted overlays fall back to the
native two-screen rendering while the slider is nonzero. The Cauldron's
existing drawing path remains separate. `ZAKURO_MENUS_TOP=0` disables this
feature without changing console state or the save-state format.

Headless captures: `trace/one-screen/{native,wide,ultra,narrow}/`. There are
29 scripted layouts plus the camera and generic dialog using existing pause
rendering. Tests include native/top hover, activation, keyboard handoff,
weapon/power scrolling, sliders, and held power dragging. Windows pointer
acceptance is still manual.

The final native/top input comparison matched all 112 recorded state words
and all 21 accepted clicks. The held power drag initially differed because
the fixed two-vblank initial touch could be missed during a replay. Keeping
that initial pixel through the measured drawing interval fixes pickup;
the resulting dropped-board screenshot is now byte-identical to native
input. Core tests cover that interval and exact native-pixel round trips.
The final core mouse suite passes 32 tests; the GPU suite passes 181 tests
with two existing benchmarks ignored. High-resolution Vulkan captures of
Solo, fusion, powers and a dialog are in `dist/one-screen-menu-screenshots/`.

Windows release rebuilt as `dist/zakuro-kiu-mouse.exe`, SHA-256
`4b61ca0642451256c3aa8378bd189693b6c6f1d6bf55d02c3f54ac48bfe089be`.
The previous executable is `dist/zakuro-kiu-mouse-before-one-screen-menus.exe`.
Screenshot gallery: `dist/one-screen-menu-screenshots/index.html`.

Validation limit: a single Vulkan process capturing Solo and then loading
fusion stalled during the second warmup. Fresh processes captured each of
Solo, fusion, powers and the dialog successfully at 3x. The four full layout
matrices and native/top input comparisons completed with software rendering.
Repeated state loading after high-resolution Vulkan rendering needs follow-up;
it is not covered by the passing single-state captures.

## Solo hub button collisions in the resized layout

The interactive buttons use their own 2D rigid-body system, not just the
menu hit-test triangles. `0x00D48E70` points to its body list (`+0xCC` next).
Each body points to its layout part at `+0x50` (part name at `+0x24`) and
native element at `+0x54`. Position is `+0x74`, vertices are linked from
`+0x58` (`+0x48` next, previous/current world points at `+0x1C/+0x28`),
and drag/spring joints are linked from `+0x64` (`+0x6C` next, world anchor
at `+0x58`). Restoring the joint anchors matters when borrowing coordinates.

`0x002F0770` resolves body pairs. Its calls at `0x002A3374/0x002A3470`
now borrow a common canvas coordinate space and restore native coordinates
on return, preserving solver changes. The common uniform scale cancels,
so each body needs the translation from its displayed placement divided by
that scale. The wall solver call at `0x002A34C8` receives expanded bounds.
The solver itself stays recompiled.

The drag joint is input-owner `+0x600`, selected body `+0x5FC`, target at
`+0x5D8`. Its old screen-to-world conversion and touch event filtering stop
long PC drags at the original screen. Touch callbacks and the physics update
(`0x0025EF84`, after its register push) maintain the virtual target from the
host pointer, with the physical HID touch kept in its valid 320x240 range.
This is scoped to Solo while one-screen menus are enabled.

Validation: a short Practice Range drag leaves the distant Arms Altar body
at x=9; a drag reaching its visible location moves it to approximately
x=12.33. The long drag reaches native target x=52.19, beyond the old screen,
and both contact and no-contact cases use physical host-pixel paths.

## Chapter select scene separation

Chapter select (menu index 1) keeps its native top scenery at full-window
size. `MenStageselectUp0000` contains both scenery and the chapter artwork:
`NL_U0000` is the backdrop, `PN_U0000`/`PN_U0001` the corner decorations,
and `PN_U0002` the chapter picture. `MenStageselectUp0001` supplies the
completion badges. Replay the picture, badges and top text proportionally
at the top, with scenery and its text pass omitted from that replay.

The bottom overlay retains `MenStageselectBt0000` (chapter thumbnails),
`Bt0002` (scrollbar), `Bt0003` (difficulty controls), `Bt0004` (completion
marks), and `Bt9999` (Back). Hide `Bt0000`'s `NN_B0000` mesh as well as
`Bt0001`; otherwise a white bottom-screen panel follows the controls.
The native chapter scrolling clips remain mapped with the control group;
background and details do not accept input. All visibility/material edits
are restored before native bottom rendering.

Chapter hover must preserve its last completed native hit until the host
pointer moves. The list scrolls when a thumbnail is outlined; repeatedly
refreshing the same hit every frame invalidated `menu_looked` at the time a
click arrived. The chapter-specific refresh exception preserves the native
menu's selection and leaves animated hub-model hit refreshes unchanged.

Checked headless on `trace/s11.state` at 5:3, 16:9 and 21:9
(`trace/chapter-layout-verify.txt`): the second thumbnail is outlined and the
list scrolled to it, Back takes the return hash `9A07E025`, and a click at
`mfrom 140 60`, inside the chapter picture, is taken and opens the Fiend's
Cauldron (`trace/chapter-click-verify.txt`).

Windows release rebuilt as `dist/zakuro-kiu-mouse.exe`, SHA-256
`072a802ccb0308459c6112b4580a16e5c19f4a7d97e3a7aad892fa9e643f7756`,
and copied to `C:\Users\Patrick\Downloads\zakuro-kiu\zakuro-kiu.exe`.
The previous executable is `dist/zakuro-kiu-mouse-before-chapter-layout.exe`.
Manual Windows testing remains.

## Button prompts in the stand-in system font

Hint strips and dialogs write the console's button symbols as Private Use
characters of the shared system font: `U+E000` A, `E001` B, `E002` X,
`E003` Y, `E004/E005` L/R, `E006` +Control Pad, `E052..E055` L/R/ZL/ZR,
`E073` HOME, `E077` Circle Pad, `E079..E07E` pad directions. French text
also uses `E068..E06A` for the ordinal superscripts "er", "re", "e". Seen in
the game's data: `"\uE000 OK   \uE001 Cancel"`, `"\uE052/\uE053: Rotate"`,
`"\uE073HOME"`, and file select's `\uE073HOME / go to HOME Menu` strip.

Without a dump, Zakuro's generated stand-in font (`services/shared_font.rs`)
had only JIS, Latin and punctuation, so every one of those characters
missed its CMAP lookup and the prompt lost its button. The stand-in now
draws them (`services/prompts.rs`), each button as a `Label` set through
`System::set_prompts`: the console's look, a keycap, a mouse button, or a
controller's button (Xbox letters, or PlayStation shapes by controller
name). The frontend picks labels from the last-used device and current
bindings, and redraws on rebinding or device switch. A redraw rewrites the
mapped block in place, only when its headers match a fresh build, so a
dumped font or an older state's differently-sized stand-in is left alone.
Symbol advance is fixed at 24 so already laid-out lines stay valid.

Validation: fresh boot to file select shows the HOME symbol with the
stand-in, matching the dump's placement; loading that state then switching
to keyboard labels showed an `Esc` keycap in the same strip two frames later.

Windows release rebuilt as `dist/zakuro-kiu-mouse.exe`, SHA-256
`442ba2bdf5e0c8cb41f1952fe21ead02ef4739800ecb5de109d5818632ad5e75`.
The previous executable is `dist/zakuro-kiu-mouse-before-prompts.exe`.
