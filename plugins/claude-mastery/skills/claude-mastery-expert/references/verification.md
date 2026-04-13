# Verification — The 2-3x Quality Multiplier

> Probably the most important thing to get great results out of Claude Code.

## The Principle

Give Claude a way to verify its work. When it has a feedback loop, quality improves 2-3x. This is not optional — it's the single highest-leverage thing you can do.

## How the Claude Code Team Verifies

For changes to claude.ai/code:
1. Claude opens a browser via Chrome extension
2. Tests the UI directly
3. Iterates until it works and the UX feels good
4. Only then submits the PR

## Verification Patterns by Output Type

### Code Changes
- Run the test suite after every change
- Run linter/formatter (oxlint, prettier, etc.)
- Run type checker (tsc --noEmit)
- Diff the output against expected behavior

### File Creation
- Read the file back after writing
- Check required fields/sections exist (verification script)
- Validate against a schema or template

### API Calls
- Check response status code
- Validate response schema matches expectations
- Confirm the side effect happened (event created, message sent)

### UI Changes
- Open browser (Playwright MCP)
- Take screenshot
- Compare visually or check DOM structure
- Iterate until it matches the spec

### Data Processing
- Compare input/output record counts
- Spot-check sample values
- Write results to file, grep for anomalies
- Run assertions on expected properties

### Meeting Summaries / Reports
```bash
#!/bin/bash
# verify.sh — Check summary has all required sections
FILE="$1"
ERRORS=0
for section in "Overview" "Key Decisions" "Action Items" "Follow-ups"; do
  grep -q "$section" "$FILE" || { echo "MISSING: $section"; ERRORS=$((ERRORS+1)); }
done
[ "$ERRORS" -gt 0 ] && exit 1 || echo "PASS: All sections present"
```

## The Verification Skill Pattern

Product verification skills are extremely useful for ensuring Claude's output is correct. It can be worth having an engineer spend a week just making your verification skills excellent.

Techniques:
- Have Claude record a video of its output (screen recording) so you see exactly what it tested
- Enforce programmatic assertions on state at each step
- Include verification scripts in the skill's `scripts/` directory
- Run verification as a hook (PostToolUse) for automatic enforcement

## Anti-Patterns

- **Fire and forget** — Agent does work, never checks the result
- **Trusting LLM output** — Agent writes JSON, doesn't validate it parses correctly
- **Skipping tests** — Agent writes code, doesn't run the test suite
- **Manual verification only** — If a human has to check, it's not scalable
