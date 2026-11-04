# Kid Icarus: Uprising PC Port

Terminology for the PC recompilation, graphics enhancement, and single-screen mouse/keyboard interface. Use these terms in issues, plans, tests, and change requests; the words under _Avoid_ are not used. Code anchors live in `analysis/FINDINGS.md`, and change requests follow `docs/agents/change-requests.md`.

## Language

### Menus

Do not write `menu` alone: every reference names one of the three families below.

**In-Game Menu**:
The pause screen, its Options pages, and the dialogs opened over a battle. Drawn by the game's battle UI.
_Avoid_: Pause menu, in-battle menu

**Scripted Menu**:
An out-of-battle game screen laid out by the game's scripted layout system: file select, chapter select, the hubs, Armory, Vault, Fiend's Cauldron.
_Avoid_: Front-end menu, out-of-battle menu, static screen, 2D scene

**Frontend Menu**:
Zakuro's own Esc overlay, drawn by the emulator and containing no game UI.
_Avoid_: Emulator menu, settings overlay, debug menu

**Screen**:
One named menu surface the game enters; listed under Screens.
_Avoid_: Page (except where the game's own page is meant), view, scene

**Dialog**:
A Screen shown over another Screen, sharing its input.
_Avoid_: Popup, modal, prompt

**Hint strip**:
The one- or two-line control hint the game draws along a Screen edge.
_Avoid_: Help text, footer, caption

**Screen Transform**:
The one process that moves a game-owned Screen's drawing and input onto the top screen: detect, capture, seat, replay, map input.
_Avoid_: Menu relocation, top-screen replay, UI hack

**Placement**:
A source rectangle on the game's screen and where it shows on the top screen, shared by drawing and hit-testing.
_Avoid_: Offset, box, viewport

### UI structure

**Board**:
The fixed layout container of a Screen, holding Controls at anchored positions.
_Avoid_: Panel, layout, canvas, scene

**Control**:
One hittable thing on a Screen: a button, row, slider, or tile.
_Avoid_: Element, widget, part, button (for anything that is not a button)

**List**:
A scrolling sequence of Rows inside a Screen.
_Avoid_: Table, grid, menu list

**Row**:
A Control that is one entry in a List.
_Avoid_: Item, line, entry

**Hover**:
The pointer lighting the Control under it through a held Synthetic Touch.
_Avoid_: Mouseover, focus, highlight

**Outline**:
The Control the game currently has selected on a Screen, lit by keys, gamepad, or Hover.
_Avoid_: Selection, highlight

**Cursor**:
The game's own selection marker moved by keys or gamepad.
_Avoid_: Pointer, mouse cursor

**Pointer**:
The host's mouse position in window space.
_Avoid_: Cursor (when the mouse is meant)

### Input

**Air Aim**:
Absolute mouse coordinate mapping used during aerial flight sections.
_Avoid_: Flight mouse, absolute look, air cursor

**Ground Look**:
Delta mouse look with camera box containment collapsed for third-person ground combat.
_Avoid_: Ground aiming, relative mouse, ground turn

**Synthetic Touch**:
Stylus touch state synthesized from the host pointer and mouse buttons (touching, began, x, y).
_Avoid_: Virtual touch, mouse emulation, fake stylus

**Pointer Reset**:
Forced broadcast of an out-of-bounds release event to clear guest touch state on window focus loss or scene change.
_Avoid_: Touch cancel, cursor release, input flush

### Rendering

**Top Screen Pass**:
The render pipeline pass that emits graphics to the primary 3D display buffer.
_Avoid_: Main screen, primary viewport, Upper LCD

**Sprite Batch**:
The shared vertex quad buffer submitted to the GPU command list.
_Avoid_: UI buffer, 2D mesh, quad array

**Lens Aspect**:
The perspective camera's aspect-ratio parameter.
_Avoid_: Camera ratio, projection aspect, frustum ratio

**Aspect Ratio Factor (k)**:
The current viewport aspect ratio divided by the original 5:3 3DS aspect.
_Avoid_: Aspect multiplier, widescreen scale, zoom factor

**Quad Squeeze**:
Horizontal scale by 1/k applied to 2D sprite quads during the sprite batch flush.
_Avoid_: UI shrink, sprite unstretch, aspect correction

### Build

**C Override**:
Hand-written native C hook compiled into the recompilation runtime instead of into the game.
_Avoid_: Patch, hack, DLL injection

## Screens

Canonical names for the Screens a change request can name. Use the game's visible label; the family determines which subsystem owns the Screen.

| Screen | Family | Notes |
| --- | --- | --- |
| Pause | In-Game Menu | Main pause page: Resume, Retreat, Options, Remove Display |
| Retreat Dialog | In-Game Menu | Confirmation over Pause |
| Control Configuration | In-Game Menu | Options page for binding controls |
| File Select | Scripted Menu | Three save books |
| Chapter Select | Scripted Menu | Chapter thumbnails and Back |
| Main Menu | Scripted Menu | Solo, StreetPass, Vault, Together, Options, AR, guide, Back |
| Options | Scripted Menu | Reached from Pause and Main Menu |
| Other | Scripted Menu | Options' Other page |
| Controls | Scripted Menu | Control Configuration, Reticle/Camera, More Controls, Back |
| Reticle/Camera | Scripted Menu | Air Battle, Land Battle, Back |
| Subtitles | Scripted Menu | Subtitle settings |
| Volume | Scripted Menu | Music, SFX, Voice, Music Gallery, Back |
| Profile | Scripted Menu | Player and Back |
| Profile Yes/No | Scripted Menu | Confirmation over Profile |
| SpotPass Dialog | Scripted Menu | Yes/No choice over Other |
| Solo Hub | Scripted Menu | Move Out, Arms Altar, Practice Range, Idol Toss, Gear, Treasure Hunt, Back |
| Arms Altar | Scripted Menu | Redeem Hearts, Fuse Weapons, Convert to Hearts, Gear, Back |
| Practice Range | Scripted Menu | Air Battle, Land Battle, Gear, Back |
| Armory — Gear | Scripted Menu | Equipment list |
| Armory — Redeem Hearts | Scripted Menu | Redeem list |
| Armory — Convert to Hearts | Scripted Menu | Convert list |
| Armory — Fuse Weapons | Scripted Menu | Fusion board |
| Armory — Powers | Scripted Menu | Power list |
| Armory — Weapons | Scripted Menu | Weapon rows |
| Armory — Categories | Scripted Menu | Category rows |
| Idol Toss | Scripted Menu | Two instances: from Solo Hub and from Vault |
| Treasure Hunt | Scripted Menu | Tile board |
| Vault | Scripted Menu | Idols, Music Gallery, Power Portrait, Offering, Idol Toss, Records, AR, Back |
| Music Gallery | Scripted Menu | Track List inside Vault |
| Fiend's Cauldron | Scripted Menu | Bet bar, pot, Move Out, Back |
