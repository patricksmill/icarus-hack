# Kid Icarus: Uprising PC Port (`icarus`)

PC recompilation and enhancement project for Kid Icarus: Uprising (USA, 0004000000030100) using `3dsrecomp` and `zakuro`.

## Key Directories & Components

- `tools/3dsrecomp`: upstream recompilation toolchain (Rust).
- `tools/zakuro`: upstream runtime environment.
- `ghidra/`: analysis scripts and headless disassembly pipelines for `code.bin`.
- `overrides/`: C overrides passed to `3dsrecomp port --overrides overrides/`.
- `docs/adr/`: architectural and porting decisions.

## Windows delivery

After code changes, rebuild the Windows release executable and replace `dist/zakuro-kiu-mouse.exe` before reporting the changes ready to use. Verify the distributed executable matches the new build; report build or runtime-verification blockers explicitly.

## Agent skills

### Issue tracker

GitHub Issues on `patricksmill/icarus-hack`. See `docs/agents/issue-tracker.md`.

### Triage labels

Canonical 5-role triage vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context repo layout (`GLOSSARY.md` + `docs/adr/`). See `docs/agents/domain.md`.

### Vocabulary

- Use `GLOSSARY.md` terms; never substitute a word marked _Avoid_.
- Every change request follows `docs/agents/change-requests.md`: Operation, Target, Screen, before/after, Acceptance.
- Every game-owned Screen follows `docs/agents/screen-transform.md`: one pipeline, one `ScreenSpec`, one `Placement` list shared by draw and hit-test.
- Code anchors stay in `analysis/FINDINGS.md`; the glossary holds concepts only.
