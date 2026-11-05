# Screen Transform

The contract every game-owned Screen follows to appear on the top screen. Vocabulary is in `GLOSSARY.md`; the decision and its alternatives are in `docs/adr/0005-screen-transform-pipeline.md`.

## Scope

Applies to every **In-Game Menu**, **Scripted Menu**, and the Fiend's Cauldron. The Frontend Menu is out of scope: it is emulator UI, not game UI.

## Stages

One orchestrator runs these stages in order. Capture happens in the bottom pass, replay in the top pass, input in the game's input path; the orchestrator owns the ordering and all state between them.

| Stage | Input | Output |
| --- | --- | --- |
| detect | frame, game state | one active `ScreenSpec` |
| capture | bottom-pass calls or scene models | recorded draw units for the Screen |
| seat | registered width and aspect | one `ScreenLayout` |
| replay | `ScreenLayout` | top-pass drawing |
| map input | pointer, keys, buttons, `ScreenLayout` | native touch or menu calls |

No stage may branch on a Screen id or registry index. Behaviour that differs between Screens is declared on the row.

## ScreenSpec

```rust
struct ScreenSpec {
    id: Screen,                 // canonical name from GLOSSARY.md
    family: Family,             // InGame | Scripted | Cauldron
    identity: Identity,         // CallSites | Vtable | TaskProbe
    first_call: usize,          // offset into a shared call table
    input: InputMode,           // PressA(addr) | TouchesOnly | ClickAsTouch
    outline: Option<OutlineFn>, // fn(&mut System, u32, Found) -> Vec<MenuCall>
    activate: Activate,         // game call or native touch
    layout: LayoutKind,         // row-driven: None, FileSelect, Chapter, Generic, Group, Idol
    wheel: Option<WheelFn>,
    fallback: &'static str,     // the env flag that restores native drawing
}
```

The registry now carries all 26 Scripted Menus plus Pause, Retreat Dialog and the Fiend's Cauldron: `id`, `family`, `identity`, `first_call`, `input`, `fallback`, `outline`, `activate` and `layout`. Pause and Retreat Dialog share one call table, so Retreat Dialog starts at `first_call = 1`. Scripted Menu layout data is row-driven (`LayoutKind`); `menu_top` still reads it through its index entry point, and the editor-facing anchor fields and `wheel` are pending.

`Identity` selects one of three detector adapters; all three are implemented:

- `CallSites(&'static [ToTop])` — pause and dialogs.
- `Vtable { vtable, update }` — Scripted Menus.
- `TaskProbe { show }` — Fiend's Cauldron.

`Screen` is a typed enum of glossary names, so a request, a test, and the editor refer to the same value.

## Placement

`ScreenLayout` is an ordered list of placed draw units:

```rust
struct Placement {
    from: [f32; 4], // source x, y, width, height in the game's screen pixels
    to: [f32; 4],   // target x, y, width, height in top-screen pixels
}

struct PlacedUnit {
    unit: DrawUnit,   // Model | CallSite | Layer | QuadGroup
    place: Placement,
    shown: bool,
}

struct ScreenLayout {
    units: Vec<PlacedUnit>,
}
```

Seat produces the list once per frame. Replay draws from it; input hit-tests the same list. A second geometry calculation for input is forbidden: that is how hover drift returns.

Pause uses five editable `PlacementRule`s keyed by `QuadRole` (span, centre, left button, right button, corner). A rule carries visibility, the source box, the target box at 400x240, an anchor, and a scale mode (`Fixed`, `Narrow`, `Stretch`); `PauseRules::layout` resolves them for the current width. Hidden rules yield no Placement for drawing or hit-testing. The editor mutates these rules, not the resolved rectangles.

The Retreat Dialog has its own two rules, `DialogRules` (span: the dimming; panel: the box, its No and Yes, and the question). `ScreenTransform::screen_layout` picks Pause's or the dialog's by the call table entry being drawn, and touch reads the dialog's while it is the active Screen, so Pause's rules never move the dialog and the dialog's never move Pause. The panel places both x and y (Pause places x only); the words, drawn through the 3D renderer, follow it through their projection (`place_projection`), and a hidden panel draws none of them. F2 on the dialog opens **Menu editor — Retreat Dialog** with Save layout, Reset and Export; `menu-layout.toml` keeps its rules under `dialog`, and older files load the defaults. `kiutrace` edits them with `dialogrule` and `dialogvisible`.

The Fiend's Cauldron uses eight editable `PlacementRule`s keyed by `CauldronRole` (top picture, panel, bar, dial, Back, Move Out, rest, ring) in `CauldronRules`. The ring (`MenBetHeartBt0010`, the bottom screen's background) takes no touches and is drawn before the top's 3D, so the board, the pot and every other part cover it. A saved list of Cauldron rules from before a role was added loads with that role's default. They use the `Fit` scale mode: the target box is authored for a 427-pixel-wide shown screen (16:9), shrinks evenly below it, and `anchor`/`anchor_y` pick the left/right and top/bottom edge it keeps; the result is in the same 400×240 top-screen space as Pause. The panel's and dial's rules are where they sit at intensity 0; `CauldronRules::layout` slides them along the bar with the pot's value, and they have no Placement without a value. The pot's task probe seats this layout once per frame; drawing, hit-testing and Cursor lighting all read it. What the pointer does over each part — the native Cursor name, the touchable sub-box (Move Out's button), and the dial's drag along the bar — is the `CAULDRON_CONTROLS` table, in hit-test order; it is not editable rule data. The frontend sends every top-screen touch through `AimMouse::top_screen_touch`, which picks Pause, the Cauldron or a Scripted Menu. `kiutrace` edits the rules with `cauldronrule` and `cauldronvisible`.

On the Cauldron, F2 opens **Menu editor — Fiend's Cauldron**, listing its eight parts instead of stale Scripted Menu layers. Drag a part to move it; its corner handle resizes it; Visible hides drawing and touch together. Panel and Dial edits subtract the current intensity's slide before storing their rules, so changing intensity keeps the edit. Canvas edits are converted back to design-width rules and reseat drawing and input immediately. **Save layout** stores Cauldron rules alongside Pause rules in `menu-layout.toml`; older files default to the committed rules. Console state loads and game resets retain edits; normal launches without `--menu-editor` ignore saved rules. Reset restores defaults, and **Export Rust + change request** writes `menu-layout-export.txt`. Pause and Retreat Dialog expose no stale Scripted Menu layers.

Anchors are layout-rule data, not runtime fields: edge-anchored Controls and centred content compute their target rects for the current width, and the resulting `Placement` is the concrete mapping. The editor edits those rules, so a layout keeps its shape at 5:3, 16:9, 21:9 and dragged widths.

### One view

The pipeline draws one view: the top screen as shown. Kid Icarus draws every top-screen pass twice, into the left eye (`0x18080000`, shown) and the right eye (`0x18000000`, never shown with the 3D off), even with the slider down. The GPU drops every draw into the right eye's target at the top screen's size (`aim_mouse::RIGHT_EYE`, set each frame while the mouse module is attached); the bottom screen reuses that VRAM at 240×320 and is kept. Replays are made once, into the shown eye (the Retreat Dialog's words at `0x007258C0`). The 3D settings are hidden and always off.

## Behaviour invariants

Every transformed Screen satisfies all eight. Exceptions are declared on the row and named in the acceptance scenario.

1. Pointer over a Control lights its Outline.
2. Pointer off every Control clears the light.
3. A click activates exactly what native A activates, or a native touch where the Screen is touch-only.
4. Press-and-hold or drag remains a native touch.
5. Keys and gamepad restore the native Cursor.
6. Wheel scrolls a List when the Screen has one.
7. Focus loss or Screen change fires a Pointer Reset.
8. Disabling the Screen's `ZAKURO_*` flag restores untouched native dual-screen drawing.

## Migration

Order: Pause, Retreat Dialog, Options family, Scripted Menus (simple button Screens first, Armory lists and boards last), Fiend's Cauldron.

For each Screen:

1. Add its `ScreenSpec` row and detector adapter.
2. Route capture, seat, replay and input through the orchestrator.
3. Keep the existing flag as rollback.
4. Prove parity (below) and delete the legacy path.

A Screen is migrated only when all five stages run through the pipeline. Partial-stage wiring counts as in progress, not done.

## Verification

- Registry test in `zakuro-core`: unique ids, every field present, every migrated Screen has a row, no routing gaps.
- One `kiutrace` scenario per Screen: save state, hover point, click, expected Outline and activation, Pointer Reset, and the fallback flag.
- Screenshots before and after a cutover at 5:3, 16:9, 21:9 and a narrow window.
- No pop-in on entry: every adapted Screen has a model-name prefix in the entrance-detection list in `scene()` (`aim_mouse/menu_top.rs`), so the new layout draws from the first frame its graphics load, before the Screen's input update runs. Check by replaying the transition into the Screen with a `shot` every few frames: the original layout must never appear.

Existing save states are the baseline; a Screen without a scenario is unported.

## Editor

Dev-only egui overlay, opened with `--menu-editor` and toggled with F2 anywhere, or by the Layout editor button in the Esc menu. It draws a canvas over the live top screen, outlines the active Scripted Menu's layers, and lists them by model name. Dragging a layer moves it; the bottom-right handle resizes it. The edit is kept per screen and part index and applied when the layout is rebuilt, so drawing and hit-testing follow it. Pause rules remain the numeric `PlacementRule` editor.

The editor opens independently of the Esc overlay. A drag selects the topmost layer under the press; its lower-right handle resizes from the original placement, and releasing ends the edit. Changing the layer list clears the previous selection. Editor windows sit above the canvas so their Controls take input, and long layer/rule lists scroll within the available window height. Both editor windows can be closed with their X; saving is available for Scripted Menu placements and for Pause rules.

Ctrl or Shift adds a layer to the selection, on the canvas or in the list, and takes a selected one out again. Dragging any selected layer moves them all; a selected layer's handle scales the whole selection about its top-left corner, keeping the layers' arrangement. A plain click or a drag on an unselected layer starts over with that one. Navigation edits the last layer picked.

Every Screen except File Select and Chapter Select also lists the game's own top-screen drawing as its last layer: **Details card** where the menu shows weapon or page details beside its Controls, otherwise **Top screen** (the scene behind a hub). Moving, scaling or hiding it changes where that native pass is placed and clipped; it takes no pointer input and has no navigation. Being last, it left every earlier layer's saved index unchanged.

Every Scripted Menu item has a **Visible** checkbox, including HOME hint, backgrounds, information Boards and Hint strips. Pause exposes the same checkbox inside each rule group. Hidden items remain in the list with their geometry and stable indices so they can be shown again; their gray editor bounds have no drag handle and cannot intercept canvas selection or dragging. Hidden Scripted Controls cannot receive pointer input or be chosen as explicit navigation destinations. Visibility changes cancel any held Synthetic Touch. Hiding is a visual layout setting, not deletion of native game state or keyboard/controller bindings.

**Save layout** persists hidden Screen/part pairs even when no geometry was moved, plus each Pause rule's visibility. Loading a console state keeps visibility; the next editor-enabled launch restores saved visibility. Older files without visibility data default to all items visible. Export includes visibility alongside geometry.

Animated model Controls use those same live placements for collision solving and drag bounds, including editor overrides; they do not rebuild default Solo Hub geometry for physics. Main Menu and other matching model Controls share that path. Held host drags retain their unbounded mapped coordinates past the native bottom screen's 320×240 touch limits. The native bottom-screen copy remains a native viewport; its clipping does not limit movement or replay on the top screen. Editing a placement invalidates cached pointer candidates so Hover is remapped on the next input update.

The `kiutrace` command `layer <index> <x> <y> <width> <height>` exercises the same editor mutation as the overlay; `layers` prints indices and placements. Use it with `mhover`, `expectmenu`, `click`, and `mdragtop` to check drawing and interaction from a save state.

`kiutrace` also supports `visible <index> <0|1>` for Scripted items and `pausevisible <span|center|left|right|corner> <0|1>` for Pause groups. Hiding every File Select item produces an empty top screen rather than falling back to the original unfiltered parts.

File Select exposes eight independent Placements: Book I, Book II, Book III, Back, How to Play, HOME hint, Player information Board, and Background. The books retain native 3D depth/material ordering; their meshes and player-name text follow the same Placement without clipping the turning covers to the editor box. Native font anchors select each book's complete name and shadow rather than copying strings or clipping neighbouring names. The HOME hint, information Board and Background are decorative, not activation or navigation targets. Leaving every Control clears the host's previous Hover result.

File Select's native black cross-screen book connectors are omitted from the transformed drawing. They are decorative meshes, not the books or their native selection/activation state, and cannot line up once the Boards and books are edited independently.

File Select's former single `controls #0` is replaced by these independent entries. Re-edit saved File Select placements made with that older grouping; other Screen entries retain their indices. Save layout and console-state reload retain the new per-Control placements through the existing layout format.

### Explicit directional links

Select an addressable model Control and use **Navigation** to set its Up, Down, Left and Right destinations. **Native default** removes just that direction's override; **Stay on this Control** is a self-link. Geometry edits do not automatically change explicit links.

Individually editable Controls and native links cover File Select, Main Menu, Solo Hub, Arms Altar, Options, Practice Range, Vault, Other, Controls/More Controls, Reticle/Camera, Subtitles, Volume, Profile and Profile Yes/No. Child Controls keep their native Board arrangement by default; each Placement crops the Control's meshes and dynamic text together. Volume's labels and slider pieces move with their slider. Controls and More Controls share stable saved indices; rows on the other page are marked **inactive**, excluded from drawing, hit-testing and navigation destinations.

The SpotPass Dialog exposes its caption, No and Yes separately. No and Yes have independent Placements and Left/Right links. Its native choice box has no Up/Down actions; the editor marks those directions unavailable rather than storing inert links. Other unbound and grouped Screens retain native navigation.

Links redirect native graph lookups, including the per-Screen inline lookups, rather than rewriting pad input. Native Outline updates and A activation remain the game's. Volume uses its native queued-name path when a link overrides a slider direction; without a link, Left/Right still adjusts the slider, while a self-link leaves its value unchanged. Diagonal directions retain their native routes. Close the editor with F2 to test game input. **Save layout** stores links alongside placements and Pause rules; older layout files default to no links. Links survive console state loads and game resets, and normal launches without `--menu-editor` ignore them.

File Select keeps native repeat and A activation, but redirects a linked cardinal action through its native Outline helper instead of only substituting a graph slot: that helper also changes the selected book index and animates the correct cover. A jump from Book I to Book III therefore changes both the Cursor and the book, while a self-link changes neither.

`kiutrace` supports `navlink <from> <up|down|left|right> <to|native>` using the indices printed by `layers`. Use `hold`, `circle`, and `expectword <address> <value>` to check the native Cursor, rather than the pointer-only `expectmenu`. The Main Menu acceptance replay redirects Solo's Right to Vault, checks all four cardinal directions and circle-pad input, checks a held self-link, removes the override to recover StreetPass, and activates Vault with native A.

While open the editor owns pointer and keyboard; it releases the aim capture and blocks the recapture path, so opening it during play does not leave the cursor held by the game. **Save layout** writes both Scripted Menu placements and Pause rules to `menu-layout.toml` in the data directory; the next start with `--menu-editor` restores both. Existing layers-only files still load. Loading a console save state clears recorded drawing and pointer state but retains host editor placements and Pause rules, including unsaved edits. Other launches ignore the layout file. A console save state and **Save layout** are separate: the former restores the game; the latter persists editor geometry.

To make a change part of the game by default, apply the exported Rust/change request to the committed descriptor. Export writes `editor-layers-export.txt` with layer geometry, visibility and change requests, and the Pause panel writes `menu-layout-export.txt` with a Rust `PauseRules` fragment. The committed descriptor remains the source of truth for normal launches. Still to come: Scripted rule generalization so edits survive every aspect width, grouping, and aspect previews.
