# Non-Coding Agents — When Claude Code Isn't Just for Code

> Source: Claude Code team practices, Anthropic official docs

## The Core Insight

Many team members at Anthropic use Claude Code as a general agent, not just for code.

Claude Code's power comes from bash + filesystem access, not from "being a coding tool." These capabilities work for any digital task.

## Why Bash Works for Everything

| Task | How bash handles it |
|---|---|
| Email processing | IMAP CLI, write emails to files, grep across them |
| Data analysis | Read CSV, write Python/Node scripts, run them, read output |
| Web research | curl + parsing, or Playwright MCP for full browser |
| File management | find, mv, cp, sort — native filesystem operations |
| Document processing | pandoc, pdftotext, imagemagick — CLI tools |
| Scheduling | cron, at — agent creates its own automation |
| API calls | curl with JSON processing via jq |

## The Email Agent Pattern

Instead of dumping 100 emails into context:
1. Fetch emails via IMAP CLI -> write to `emails/` directory
2. Agent greps across email files to find relevant ones
3. Agent reads specific emails it identified
4. Agent processes and responds

**Why this works:** Multiple passes at the problem. First pass: broad search. Second pass: detailed read. Third pass: synthesis.

## The File-First Pattern

The file system is an elegant way of representing state that your agent could read into context and verify its work.

For any non-coding agent:

```
workspace/
  inbox/           # Raw data from external sources
  processed/       # Agent's working files
  output/          # Final results
  MEMORY.md        # What the agent knows
  scratch/         # Temporary work
```

The agent writes intermediate results to files, searches them, processes them. The filesystem IS the agent's memory and workspace.

## Practical Non-Coding Skills

### Meeting Intelligence
```
Agent: meeting-cli search "pricing" --json > scratch/meetings.json
Agent: cat scratch/meetings.json | jq '.[] | .title + " - " + .date'
Agent: meeting-cli get <id> --json > scratch/meeting-detail.json
Agent: (synthesizes answer from files)
```

### Data Analysis
```
Agent: Write a Python script to analyze the CSV
Agent: python analyze.py > scratch/results.json
Agent: (reads results, interprets, responds)
```

### Report Generation
```
Agent: (gathers data from multiple sources -> files)
Agent: (writes report to scratch/report.md)
Agent: bash scripts/verify.sh scratch/report.md
Agent: (if PASS, delivers report; if FAIL, iterates)
```

## Key Design Principles for Non-Coding Agents

1. **Give bash access.** This is non-negotiable. It's the universal tool.
2. **Use the filesystem for state.** Don't try to hold everything in context.
3. **Write scripts for repeatable tasks.** Agent generates scripts, runs them, reads output.
4. **Multiple passes over data.** Write -> search -> filter -> process. Not one giant prompt.
5. **Verify via code.** Write assertions, run checks, compare counts. Don't trust LLM reasoning alone.
6. **CLI tools over APIs.** If a service has a CLI, use it. CLIs handle auth, formatting, pagination.

## The Spec-Based Development Pattern

For complex non-coding projects:

**Phase 1: Requirements Interview**
```
"Ask me detailed questions about this project using the AskUserQuestion tool.
Be very in-depth and continue interviewing me until requirements are complete."
```

**Phase 2: Lock In SPEC.md**
```
"Document all decisions in SPEC.md. This is the source of truth."
```

**Phase 3: Plan Before Execution**
```
"Enter Plan Mode. Generate a detailed execution plan. I'll review before you start."
```

This works for any domain: marketing campaigns, data pipelines, business processes, research projects.
