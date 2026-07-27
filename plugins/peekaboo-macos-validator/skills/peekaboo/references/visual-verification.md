# Visual verification

Use this reference after capturing a meaningful application state. A click
proves only that an input path executed; it does not prove the product state is
correct, legible, intentional, or resilient.

## Critique contract

Treat every screenshot as large. Give the visual reader the minimum artifact
paths, the state or comparison being judged, and the rubric below. Keep image
payloads out of the driver and return only a compact text report. Immediately
after the report, terminate the reader with the host's kill/stop/remove control
and verify through worker listing that it is gone. If those controls are
unavailable, do not create the reader.

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

Prefer a compact `PASS`, `WARN`, or `FAIL` table with evidence and an intent
verdict. Numeric scores are optional and need an anchored rubric:

```markdown
**Critique - Settings panel**

| Dimension        | Verdict | Evidence |
|------------------|---------|----------|
| Intent           | WARN    | The grouping is clearer, but Save is visually weak. |
| Hierarchy        | WARN    | Section title and secondary help compete at equal weight. |
| Alignment        | PASS    | Toggle rows align to one grid. |
| Spacing          | WARN    | The final two rows read as one group. |
| Contrast         | PASS    | Critical copy is legible in this appearance. |
| Copy             | WARN    | One label truncates at the minimum width. |

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

For each state, assert an AX/product postcondition and request pixel inspection
only when it can reveal something the structured state cannot. A different
screenshot hash or snapshot ID is not itself a meaningful postcondition.

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
- Track every task-created artifact path.
- Create both raw and annotated captures only when each answers a distinct
  question; delete either as soon as its purpose is complete.
- Keep artifact directories out of version control.
- Do not retain credentials or private user content in evidence.
- Keep before/after captures at the same window geometry and appearance.
- Before the final response, delete temporary screenshots, JSON, traces, videos,
  contact sheets, and empty artifact directories by exact path.
- Retain and report exact paths only for user-requested artifacts or necessary
  failure evidence.

When a snapshot exposes a specialized issue, invoke the matching available
design, accessibility, typography, color, layout, or SwiftUI review skill,
then return to this verification loop after applying the fix.
