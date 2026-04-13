# Subagents — Parallel Execution & Specialization

> Source: Claude Code team practices, Anthropic official docs

## When to Use Subagents

| Use case | Why subagent is better |
|---|---|
| Task needs different model | Each subagent gets its own cache. No cache break. |
| Task produces lots of output | Isolated context. Only summary returns to main. |
| Multiple independent tasks | True parallelism. N tasks in N subagents simultaneously. |
| Risky or exploratory task | Isolation. Failure doesn't pollute main session. |
| Code review | Fresh perspective. No prior assumptions from main session. |

## When NOT to Use

- Simple questions (one search + one answer) — overhead not worth it
- Sequential work depending on previous steps — subagent can't see main context
- When cache switching cost > benefit (short sessions)

## Production Subagent Patterns

- **code-simplifier** — Runs after implementation. Reviews for reuse, quality, efficiency.
- **verify-app** — End-to-end tests with detailed instructions for testing Claude Code.
- **adversarial-review** — Spawns fresh-eyes subagent to critique. Iterates until findings degrade to nitpicks.
- **build-validator** — Ensures build passes before PR.

## Model Switching via Subagents

If you're 100K tokens into an Opus conversation and want a simple answer, it's actually more expensive to switch to Haiku than to have Opus answer — because you'd rebuild the cache.

**Solution:** Main session on Opus prepares a "handoff" message with just the needed context. Subagent on Haiku handles the sub-task with its own fresh cache.

Claude Code's Explore agents use this pattern — Haiku for lightweight codebase exploration.

## Task Coordination

Tasks replaced the older Todo system for subagent coordination:
- Tasks support dependencies (DAGs)
- Shared updates across subagents
- Can be altered, deleted, reordered
- Persistent across sessions and context clears

Store the plan on disk. Subagents read tasks from files. Main agent checks progress.

## Agent Teams (Experimental)

Direct communication between "teammates" without going through a lead:
```
Team Lead
├── Teammate A (frontend)
├── Teammate B (backend)  ← can talk directly to A
└── Teammate C (tests)    ← can talk directly to A and B
```

Enable: `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`

Use for: parallel domain work, competing hypotheses, cross-layer changes.
Don't use for: same-file edits (conflicts), sequential work, high interdependency.

## The "Use Subagents" Pattern

Tip: append "use subagents" to any request where you want Claude to throw more compute at the problem. Claude will decompose and parallelize automatically.
