# 4. Compatibility Fallback and Pointer Lifecycle Policy

Single-screen play with edge-anchored relocated menus is the default PC experience. However, headless regression tests, disassembly verification, and multi-monitor setups need access to the unmodified dual-screen drawing. Furthermore, synthetic mouse touch emulation can leave stuck touches in guest memory during OS focus changes or scene loads.

We decided to:
1. Provide a runtime opt-out flag `ZAKURO_MENUS_TOP=0` that bypasses pass-level UI relocation in `0x0092E4F8`, drawing to native top and bottom framebuffers.
2. Enforce a strict Pointer Reset policy: whenever the window loses focus (`WM_KILLFOCUS` / SDL focus lost) or a level load/scene change occurs, the host immediately dispatches an out-of-bounds release event (`touching=0, began=0, x=-1, y=-1`) to prevent stuck held-touch states.
