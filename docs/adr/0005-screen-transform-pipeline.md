# 5. Screen Transform Pipeline

Game-owned menus reach the top screen through three unrelated mechanisms: the pause menu and its dialogs replay a fixed table of 15 bottom-pass call sites with a 40-pixel offset; the 26 Scripted Menus move scene models by a per-screen registry with 18 outline handlers and index-branching layout; the Fiend's Cauldron runs its own task-loop probe and `CauldronLayout`. The three differ in detection, draw path, input semantics and layout, so every fix and every new menu tends to add a fourth variation.

We decided to collapse them into one process, **Screen Transform**: fixed stages (detect, capture, seat, replay, map input), three detector adapters behind one interface (call sites, object vtable, task probe), one `ScreenSpec` row per Screen, and one `Placement` list shared by drawing and hit-testing. Per-Screen differences become declared strategies — input mode, outline hook, activation hook, wheel support — and shared code never branches on Screen identity or index. Eight behaviour invariants (hover lights the Control under the pointer, click matches native activation, hold/drag stays a native touch, keys restore the native cursor, wheel scrolls a List, focus loss fires a Pointer Reset, fallback draws natively, exceptions are declared) form the acceptance contract.

## Considered options

- **Keep the three mechanisms.** Rejected: it is the current state, and each variation already costs a special case in shared code.
- **One generic interception point.** Rejected: the game exposes three different signals. Pause is known call sites, Scripted Menus are vtables, the Cauldron is a task probe. A single detector would need fragile heuristics.
- **A trait per Screen implementing every stage.** Rejected: each Screen would reimplement stage order and behaviour, which is the divergence the pipeline exists to remove.

## Consequences

Migration is per-Screen and flag-gated, in order: Pause, Retreat Dialog, Options family, Scripted Menus, Fiend's Cauldron. A migrated Screen keeps its existing `ZAKURO_*` flag as rollback until parity passes; then its legacy path is deleted, with no permanent dual process. A dev-only egui overlay edits `Placement` geometry, anchors and visibility and exports a Rust descriptor fragment plus a canonical change request; committed Rust remains the only source of truth and release builds read no layout file. `GLOSSARY.md` gains **Screen Transform** and **Placement** when the pipeline exists; the rules live in `docs/agents/screen-transform.md`.
