# The Agent Loop — Gather, Act, Verify

> Source: Claude Agent SDK patterns, official Anthropic docs

## Phase 1: Gather Context

The agent builds its own understanding before taking action.

### Agentic Search First
Use bash commands (`grep`, `find`, `ls`, `tail`) before semantic/vector search:
- Cheaper (no embedding model call)
- Faster (local filesystem)
- More precise (exact string match)
- Claude is good at composing bash search commands

```bash
# Agent discovers codebase structure
find src -name "*.ts" | head -20
grep -r "createSession" src/ --include="*.ts" -l
```

### Progressive Disclosure
Don't dump everything upfront. Let the agent discover what it needs:
1. Skill names and descriptions loaded at session start (lightweight)
2. Full skill content loaded when agent decides to use it
3. Reference files read on demand by the agent
4. Data files written to disk, grepped when needed

Agents get dumber when you give them too much information upfront.

### Subagents for Parallel Context Gathering
Spin up subagents to explore different parts of the codebase simultaneously. Each has isolated context. Only summaries return to the main session.

## Phase 2: Take Action

### Tool Priority

| Priority | Tool type | Use when |
|---|---|---|
| 1 | Built-in (read, write, edit, bash) | 80% of tasks |
| 2 | CLI tools via bash | External services with CLIs |
| 3 | MCP servers | Standardized external service connections |
| 4 | Custom tools (registerTool) | Bash genuinely can't do it |

### Bash Is Universal
The advice generally boils down to: use the bash tool more.

Works for non-coding tasks too:
- Read CSVs, Excel files
- Search the web (curl + parsing)
- Build visualizations (generate HTML)
- Process data (jq, awk, sort, uniq)
- File management (organize, rename, archive)

### Generate Code for Precision
When the task needs exact, repeatable execution, have the agent write a script:
```
Agent writes: scripts/analyze-meetings.py
Agent runs:   bash("python scripts/analyze-meetings.py")
Agent reads:  output file with structured results
```

This is grounded in reproducible code, not just LLM reasoning.

## Phase 3: Verify Work

Every action should have a verification step.

### Explicit Rules with Feedback
- Run linter after code changes
- Run tests after implementation
- Validate JSON/schema after generation
- Check file exists after creation

### Visual Feedback
- Playwright for UI verification
- Screenshots for visual comparison
- Video recording of test execution

### LLM-as-Judge
For fuzzy evaluation (quality of writing, design, analysis):
- Spawn a separate subagent to critique
- Iterate until findings degrade to nitpicks
- Use a different model for independent evaluation

### The Verification Script Pattern
Include `scripts/verify.sh` in every skill that produces output. The agent runs it automatically and iterates if it fails.

## The Filesystem as Memory

Between phases, the agent uses the filesystem:

```
scratch/              Temporary working files (current task)
MEMORY.md             Durable facts (survive compaction)
goals/*.md            Persistent goals (survive sessions)
data/log.jsonl        Full conversation history (searchable via grep)
```

**The pattern:** Write intermediate results to files. Grep them. Process them. Don't try to hold 100 items in context — write to disk and search.

## Context Management

### Compaction
When context fills, the system summarizes old messages and continues with the summary. Recent messages stay in full. The full history remains in log.jsonl for grep.

### Tasks for Coordination
Before Tasks: clearing context was dangerous (wiped the agent's memory of the plan).
Now: plan stored on disk as tasks. Users can `/clear` or `/compact` freely without losing the roadmap.

### The "Compaction Buffer"
Always save room in the context window for:
- The compaction instruction message
- The summary output tokens
- A few turns of conversation after compaction
