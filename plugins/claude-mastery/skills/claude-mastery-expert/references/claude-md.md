# CLAUDE.md — Your Agent's Onboarding Guide

> Source: Anthropic official docs, Claude Code team practices

## The Rules

1. **< 200 lines.** One engineer had 847 lines and got WORSE results than a 100-line version. Claude ignores bloated context.
2. **Update on every mistake.** After correcting Claude, end with: "Update CLAUDE.md so this doesn't happen again."
3. **Treat it like code.** Review changes, prune regularly, test behavior differences.
4. **Focus on what challenges Claude's defaults.** Don't restate what Claude already knows about programming.
5. **Whole team contributes.** Check into git. Updated multiple times per week.
6. **Use `@import` for deep content.** Keep CLAUDE.md concise, reference detailed docs.

## Recommended Structure (~100-200 lines)

```markdown
# Project Name

## What This Is
One paragraph. What it is, why it exists.

## Tech Stack
Languages, frameworks, key deps. One line each.

## Build & Test
The exact commands. Nothing Claude has to guess.

## Architecture
Key directories and what lives in each.
Entry points. Data flow direction.

## Conventions
- Naming patterns
- File organization rules
- Import conventions
- Testing expectations

## Anti-Patterns (Don't Do)
- Don't use X library (use Y instead)
- Don't put logic in route handlers
- Don't commit .env files

## Gotchas (Updated Continuously)
- API X returns dates in UTC, not local
- Test DB must be reset between runs
- Package Y has a known bug with Z
```

## What NOT to Include

- Generic programming knowledge ("use meaningful variable names")
- Entire API documentation (link to it instead)
- Step-by-step tutorials (that's what skills are for)
- Credentials or secrets (use env vars)
- Speculative architecture ("someday we might...")

## Per-Directory CLAUDE.md

For large projects, add focused CLAUDE.md files in subdirectories:

```
CLAUDE.md                # Root: architecture overview, tech stack
src/core/CLAUDE.md       # Core: service patterns, session factory flow
src/web/CLAUDE.md        # Web: API boundary rules, component patterns
skills/CLAUDE.md         # Skills: how to write a skill for this project
```

Each is loaded when Claude works in that directory. Progressive disclosure.

## Measuring Effectiveness

- Track how often Claude makes the same mistake twice (should be ~0 after adding to gotchas)
- Track how many corrections per session (should decrease over time)
- Review CLAUDE.md monthly, prune stale entries
- If Claude ignores an instruction, it's probably buried — move it up or simplify
