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

## Bottom-screen UI inventory (from your save states, PLAN phase 1)

Sprite flush callers (return addresses into 0x0092F408), by element:

| Caller | Element | Screen | Notes |
| --- | --- | --- | --- |
| 0x0060DB64 | narration band, 0..320 x 0..56 | bottom | dark strip behind the text |
| 0x0060DB98 | **narration text**, 2 quads | bottom | shadow (colour BF000000, +1,+1), then text (FFFFFFFF), same texture region: one texture holds the whole rendered line |
| 0x009E3068 | **portrait**, one 256x256 quad | bottom | inside 0x009E0000.. which `top_hud` treats as top HUD; only skipped by the height check, tighten it |
| 0x00A192E4 | chapter title text | bottom | not the narration |
| 0x00A1950C, 0x00A196C0, 0x00A19748, 0x00A181D0 | hearts counter and icons | bottom | |
| 0x00A11128-0x00A11CE8 | item box | bottom | |
| 0x00A13A74, 0x00A13B68 | power / special weapon icon | bottom | |
| 0x00A17E70-0x00A181C0, 0x00A0F3xx-0x00A0F6xx | background, frame | bottom | |
| 0x0060D68C, 0x00A0D22C-0x00A0E338, 0x00A04710-0x00A04F00, 0x00A01A4C | pause screen | top label, bottom menu | |
| 0x00A0E9D8, 0x00A0EA60, 0x00A182C4 | boss intro bottom screen | bottom | |

Quad positions are in the element's own space (the narration's text starts at
0,0), placed on screen by a transform elsewhere, not baked into the vertices.

Stacked narration: in your state 1 the line's texture region holds two lines'
glyphs, drawn over each other, so the line texture was not cleared before the
new line was rendered into it. Same with the 60 FPS patch undone afterwards
(the texture was already drawn). Cause to find in the text rendering path
(PLAN phase 2, question 4).
