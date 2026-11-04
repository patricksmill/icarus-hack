# Change Requests

How to ask for a change in this repo so it is unambiguous on the first read. Every request uses the vocabulary in `GLOSSARY.md` and names one Screen and one target.

## Grammar

```
<Operation> <Target> on <Screen>: <before> → <after>.
Acceptance: <observable check>.
```

All five parts are required: Operation, Target, Screen, before/after, Acceptance. A request missing the Screen or the target is answered with a question, not an implementation.

The target is a Control, or a named Screen part when the whole part moves: Board, Hint strip, or Dialog.

- Name the family (`In-Game Menu`, `Scripted Menu`, `Frontend Menu`) when the Screen name alone could fit more than one.
- "Menu" is not a target. Name the Screen from the Screens table.
- Use one Screen per request unless the request explicitly lists several Screens.
- Put no addresses in a request. Anchors live in `analysis/FINDINGS.md`.

## Operations

Closed set. Extend it only when a request genuinely fits none of these.

| Operation | Means | Example |
| --- | --- | --- |
| add | Introduce a Control that does not exist | Add a Reset button on Control Configuration |
| remove | Delete a Control and its input path | Remove the Hint strip on Vault |
| move | Change a Control's position or anchor | Move the Hint strip on Chapter Select to the top edge |
| resize | Change a Control's size without changing its content | Resize the Hint strip on Vault to two lines |
| restyle | Change appearance only: artwork, colour, emphasis | Restyle the Hover on Main Menu to a brighter outline |
| relabel | Change displayed text or artwork label | Relabel the Back Control on Armory — Gear |
| rebind | Change which input activates a Control | Rebind the List on Music Gallery to the mouse wheel |
| reorder | Change the order of Rows in a List or Board | Reorder the Rows on Options |
| hide | Stop drawing a Control while keeping its game state | Hide the Hint strip on Pause |

## Acceptance

Acceptance is one observable check, in the form a test can run: a screenshot state, a harness command, or a named scene. "Looks better" is not acceptance; "wheel from track 0 reaches track 5 and back on Music Gallery" is.

## Examples

Bad: "Fix the menu."

Good:

```
Rebind the List on Music Gallery: the mouse wheel does nothing → the wheel moves the selected Row one step.
Acceptance: on a Music Gallery save state, five wheel-down steps move the pick from row 0 to row 5, and five wheel-up steps return it.
```

```
Move the Hint strip on Chapter Select: bottom edge → top edge.
Acceptance: a top-screen screenshot shows the strip along the top edge and no Control underneath it moves.
```

## Missing terms

If a request needs a term the glossary lacks, describe the thing and where it sits on the Screen. After the change lands, add the term to `GLOSSARY.md`. Do not invent a synonym for a term the glossary already defines.
