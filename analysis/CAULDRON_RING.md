# Fiend's Cauldron ring background (`Bt0010`)

## Findings

### Draw path and representation

`Bt0010` is a 3D scene model, not a Sprite Batch. In `trace/s12.state`, the scene pointer is `0x149A6378`; `scene+0xEC` is the layer array at `0x149C1C58`. Ghidra's `FUN_00464308` (`analysis/decomp.c:547895-547903`) computes a layer as `scene+0xEC + index*0x28`. Layer `0xF` is therefore at `0x149C1EB0`; its model-list object is `0x149C3898`, and the list range is `0x149C3900..0x149C3940`. The sixth pointer, `0x149C3914`, is model `0x1405B040`.

Trace commands and observed output:

```text
load trace/s12.state
run 2
peek 0x149A6378 8
peek 0x149A6464 8
peek 0x149C1EB0 8
peek 0x149C3898 10
peek 0x149C3900 16
```

The output showed scene layer-array pointer `0x149C1C58`, layer `0xF` list pointer `0x149C3898`, list start/end `0x149C3900`/`0x149C3940`, and model pointer `0x1405B040` at list index 5. At runtime that model has vtable `0x00D14AF4` and CMDL pointer `0x15B42CA0`; the CMDL name at `0x15B45808` is `MenBetHeartBt0010`. The same CMDL dump has three mesh nodes (`base01`, `base00`, `base02`) and materials `lambert11`, `lambert12`, `lambert29`:

```text
load trace/s12.state
run 2
peek 0x1405B040 8
peek 0x1405B1BC 4
dump 0x15B42CA0 65536 bt0010
```

Observed `0x1405B048 = 0x15B42CA0`; the model node range is `0x1405B4CC..0x1405B4E4` (three 8-byte entries, all visibility flags `1` in this state). `strings -t x` on the dump showed `MenBetHeartBt0010`, `MenBetHeartBt0010__00`, the three `MeshNodeVisibilities` names and the three material names.

The game builds the draw through its scene/model renderer:

- `FUN_007c8ecc` (`0x007C8ECC`; `analysis/decomp.c:1135587-1135695`) sets the bottom viewport/camera, obtains layers `0xF`, `0x10`, `0x11`, `0x12` using `FUN_00464308`, then draws each layer's model list. `FUN_00464308` is the layer-address calculation above.
- The layer-list vtable entry is `0x008FE548` (`analysis/decomp.c:1361115-1361127`): it visits the list at `+0x18..+0x1C` and calls each model's vtable `+0x10`.
- The scene-model draw entry is `0x008E0610` (`analysis/decomp.c:1340272-1340284`); it submits the model through the renderer and visits any child models. For this model, the CMDL and runtime node array identify a single model instance with three mesh nodes. The renderer later processes mesh/node visibility (`FUN_008f5fb4`, `analysis/decomp.c:1354848-1354886`).

So the useful distinction is: the Cauldron is a **Scripted Menu** whose scene is named `SceneSettingMenuBetHeart` (`analysis/strings.txt:14129`), but `MenBetHeartBt0010` is drawn as a model in scene layer `0xF`. It is not an independently captured layout call or a **Control**. The runtime layout parameter table at `0x14B91158` contains names such as `Level_Bar`, `return`, `Saturn_Pot`, `Saturn_Dish`, `UpBG`, `Heart_Bottom`, and `Heart_Bag`; a dump of that table does not contain `Bt0010` as one of those named parts.

A `quads 2` trace on `trace/s12.state` showed only the dialog-context corner icon: caller `0x00725828`, batch `0x14A1340C`, one quad at `(382..400, 0..13)` per frame. There was no ring draw in the Sprite Batch trace. The ring's draw is in the 3D scene/model path above, not one batch or several batches. It is one CMDL model submission with three mesh nodes; the exact number of GPU primitive draws per mesh is not established here.

### Why it is currently left off

Zakuro explicitly identifies the sole excluded model as the bottom-screen background: `CAULDRON_LEFT_OFF = ["MenBetHeartBt0010"]` (`tools/zakuro/crates/zakuro-core/src/aim_mouse.rs:815-817`). The replay groups at lines 822-826 contain the bar/tag/crown, Back, and Move Out models; every other model falls into the fourth (`Rest`) group at lines 2399-2402. The `Rest` `PlacementRule` uses source `[0,0,320,240]` and target `[188,47,237.4,178.1]` (`tools/zakuro/crates/zakuro-core/src/aim_mouse/screen_transform.rs:827-830`), which is appropriate for the pot/pan but not a full-screen background. `top_screen_cauldron` replays Bar, Dial, Back, Move Out, then Rest (`aim_mouse.rs:2354-2361`); it filters the model lists and calls the bottom scene draw for each part (`aim_mouse.rs:2385-2411`).

The exclusion was tested at runtime by changing only the CMDL name's final word from `0010` to `0011` (so it no longer matches the exclusion string); this makes it fall into `Rest`. The native bottom scene remains intact, while the top replay shows the background cropped into the Rest box. Command:

```text
load trace/s12.state
poke 0x15B45814 0x31313030
cauldron
run 4
shot ring_in_rest
```

`0x31313030` is the little-endian word `0011`. Comparing that top capture with the normal cauldron capture changed pixels in `x=177..398, y=59..225`; a sample at `(200,100)` changed to near-black `(3,0,0)`, inside the placed top-picture rectangle. The ring/background consequently paints over the board when naively included in `Rest`. This directly supports keeping it out of that group. The part must have its own Placement and draw order, rather than being added to the existing pot/pan pass.

### Hook and Screen Transform work needed

The existing per-part replay machinery is already close to what is needed: `top_screen_cauldron` is reached from the hook at `TOP_SCENE_DRAWN = 0x0072A09C` (`aim_mouse.rs:782-783`, hook dispatch at `aim_mouse.rs:1850`); it calls `cauldron_viewport`, crops the bottom camera projection for `place.from`, filters models by group, then invokes `BOTTOM_SCENE_DRAW = 0x007C8ECC` (`aim_mouse.rs:2364-2411`). The bottom projection currently uses `stereo=false` (`aim_mouse.rs:2378-2383`). A new `CauldronRole` plus `PlacementRule` could therefore give the ring its own source/target rectangles, editor visibility and seat. The role/layout storage is currently fixed at seven (`screen_transform.rs:499-511, 714-730, 791-795, 852-879`), so it must grow to eight, with the default rules and editor's role lists updated. The `CAULDRON_PARTS`/`BOTTOM_PARTS` grouping needs a dedicated ring group; remove `MenBetHeartBt0010` from `CAULDRON_LEFT_OFF`. It is decorative and should not become a Control or take input (`CAULDRON_CONTROLS` is the hit/lighting list at `screen_transform.rs:781-789`).

There is an important draw-order constraint. The current hook is after the top's 3D; the source comments describe the Cauldron replay as “over the top's 3D” (`aim_mouse.rs:2309-2314`). A new ring pass there would still cover the already-drawn board wherever its target overlaps it (as observed above). To have it truly behind the board, draw the ring before the top 3D at `TOP_SCENE_3D = 0x0072A07C`, where `top_scene_viewport` currently prepares the top viewport/picture (`aim_mouse.rs:782, 2737-2769`), then let the native top 3D draw over the ring. [INFERENCE] That hook is a viable place for a ring-only bottom-scene replay, but it needs a new saved-drawing stage and careful save/restore of the current viewport, camera projection and renderer state. The simpler alternative is to keep the existing hook and choose a source/target crop that does not overlap the board, accepting that only part of the ring can be shown.

### Risks and separate frame issue

- **3D/depth and ordering:** Bt0010 is a 3D CMDL rendered with the bottom camera. The current top replay crops that camera's projection (`cropped_projection`, `aim_mouse.rs:2633-2662`) and uses a separate viewport per Placement. The runtime inclusion experiment showed it can overwrite the board; do not assume the top model's depth buffer will place it behind the board. A background pass before the top 3D, or a non-overlapping crop, is needed. Aspect mismatch between the ring's source crop and target rectangle can distort its perspective; preserve the crop's aspect with the existing `Fit` rule where possible.
- **Stereo:** the top scene's 3D path draws its layers into both eyes (`FUN_00727c24`, `analysis/decomp.c:1029820-1029960`); the Cauldron bottom replay uses `stereo=false` and is one bottom-scene draw per placed part (`aim_mouse.rs:2381-2383`, `FUN_007c8ecc`). The ring would inherit the existing Cauldron replay's mono behavior unless the new pass is explicitly drawn per eye. Whether this is acceptable at nonzero 3D slider is unverified.
- **Frame around the hearts box:** this is not part of Bt0010. `MenBetheartUp0000` is the top picture; its CMDL contains nodes `polySurface45` and `polySurface51` (runtime dump at `0x15B54920`). Zakuro labels those nodes as the dark frame around the board **and** hearts box plus shading (`aim_mouse.rs:828-837`), then hides both while placing the picture (`aim_mouse.rs:2761-2764`). That is why the native hearts-box frame is lost in the transformed view; adding Bt0010 will not restore it. Re-showing `polySurface45` may bring the frame with the placed picture, but the node spans both the board and hearts-box frame, so its alignment/clipping needs a separate decision and test.
- **Animation and scene state:** the Bt0010 CMDL has skeletal, visibility and material animation tracks; the node flags were all visible in `s12.state`, but other intensities and the pour may alter them. A new pass must reuse the live model and its current animation state rather than clone or freeze the resource.

## Recommended approach

Add an eighth, decorative `CauldronRole::Ring` with an explicit `PlacementRule` and a dedicated model group for `MenBetHeartBt0010`. Keep it out of input hit-testing. Prefer replaying it before the top 3D at `0x0072A07C` so the board and its live top-screen models naturally draw over it; preserve the ring's 3D source aspect and restore all modified scene/camera/renderer state before the native top pass continues. If that pre-3D replay proves too intrusive, use the existing post-3D replay only with a source crop and target that avoids the board, and document the cropped result. Keep the hearts-box frame decision separate: ring placement does not solve the hidden `polySurface45` frame.

## Decisions (4 November 2026)

1. The whole 320×240 backdrop, drawn behind the board.
2. One picture for both eyes, as for the rest of the Cauldron.
3. The hearts-box frame (`polySurface45`/`polySurface51`) is a separate fix.
4. The rings brighten as the intensity rises; the live model is drawn, so they follow it.
5. By default the ring sits behind the pot, in the same box as `rest`.

Implemented as `CauldronRole::Ring`, drawn at `TOP_SCENE_3D` (`top_scene_ring` in `aim_mouse.rs`) before the top's picture.
