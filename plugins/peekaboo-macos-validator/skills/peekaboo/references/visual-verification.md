# Visual verification

Use this reference after capturing a meaningful application state. A click
proves only that an input path executed; it does not prove the product state is
correct, legible, intentional, or resilient.

## Critique contract

Read the annotated PNG with the agent's image-viewing tool and answer:

- **Intent:** Did the change visibly achieve what the user requested?
- **Hierarchy:** Does attention land on the primary information/action first?
- **Alignment:** Do edges and baselines follow a coherent grid?
- **Spacing:** Are related elements grouped and unrelated sections separated?
- **Contrast:** Is important copy legible in enabled, disabled, light, and dark
  appearances?
- **Copy:** Are labels clear, concise, non-truncated, and consistent?
- **States:** Do empty, loading, error, overflow, focus, selected, and disabled
  states remain understandable?
- **Product fit:** Does the screen feel native to the app rather than like a
  generic generated dashboard?

Report evidence, not adjectives. Name the exact control/region, observable
problem, user impact, and proposed fix.

## Output shape

Use a compact scored table followed by ranked fixes:

```markdown
**Critique - Settings panel**

| Dimension        | Score | Evidence |
|------------------|------:|----------|
| Intent           |   4/5 | The new grouping is clearer, but Save is visually weak. |
| Hierarchy        |   3/5 | Section title and secondary help compete at equal weight. |
| Alignment        |   4/5 | Toggle rows align; footer misses the grid by 4 px. |
| Spacing          |   3/5 | The final two rows read as one group. |
| Contrast         |   5/5 | Critical copy remains legible in this appearance. |
| Copy             |   4/5 | One label truncates at the minimum width. |

Top fixes:
1. Increase the primary Save action's visual emphasis.
2. Add one spacing unit before the final section.
3. Shorten the truncated label.

Intent verdict: Mostly achieved; recheck after fixes at minimum width.
```

Do not invent exact pixel values, contrast ratios, or opacity percentages from
visual inspection. Measure them from code or a dedicated tool when precision
matters.

## State coverage

Choose states from the feature's user lifecycle rather than clicking every
control indiscriminately:

1. Entry/initial state.
2. Primary happy path.
3. Empty or first-run state.
4. Loading/progress state when visible long enough to matter.
5. Recoverable error and validation feedback.
6. Disabled/destructive confirmation states.
7. Long content, localization, and narrow-window overflow.
8. Final success state and any persisted result.

For each state, assert an AX/product postcondition and inspect pixels. A
different screenshot hash or snapshot ID is not itself a meaningful
postcondition.

## Responsive checks

Use `window set-bounds`, read back the actual frame, and capture representative
sizes rather than assuming the requested bounds were honored:

```bash
peekaboo window set-bounds --app "$BID" \
  --x 0 --y 0 --width 1024 --height 640
peekaboo see --app "$BID" --json --annotate \
  --path "$ARTIFACT_DIR/1024x640.png" \
  > "$ARTIFACT_DIR/1024x640.json"
```

Cover the supported minimum, a normal working size, and a large size. Inspect
sidebar collapse, toolbar crowding, text truncation, scroll ownership, sheet
placement, and minimum-size clamping.

## Transition evidence

For animation, focus transfer, or intermittent visual states, prefer
`capture action` over launching a background recording and racing the action:

```bash
peekaboo capture action --app "$BID" --duration-limit 10 \
  --pre-roll-ms 250 --post-roll-ms 800 \
  --path "$ARTIFACT_DIR/transition" --json -- \
  <action command>
```

Check the child exit result, capture warnings, `metadata.json`, contact sheet,
and optional MP4. Look for dropped/blank frames, jank, focus theft, flashes,
incorrect intermediate layout, and missing final-state settle.

## Artifact discipline

- Give files semantic names tied to a state or step.
- Keep raw and annotated captures; use annotated for targeting and raw for
  visual judgment when labels obscure the interface.
- Do not retain credentials or private user content in evidence.
- Keep before/after captures at the same window geometry and appearance.
- Report the exact artifact path for failures so another agent or human can
  inspect the same evidence.

When a snapshot exposes a specialized issue, invoke the matching available
design, accessibility, typography, color, layout, or SwiftUI review skill,
then return to this verification loop after applying the fix.
