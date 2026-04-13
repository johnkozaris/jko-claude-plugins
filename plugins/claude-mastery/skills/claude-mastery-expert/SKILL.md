---
name: claude-mastery-expert
description: This skill should be activated when the user wants to build an agent, design tools for an agent, write skills, structure a CLAUDE.md, optimize prompt caching, set up worktrees, use subagents, design verification loops, write hooks, structure a project for Claude Code, or improve their Claude Code workflow. Relevant when the user says "how should I structure this", "write a skill for", "design this agent", "optimize for caching", "set up verification", "use subagents", "parallel development", "write a CLAUDE.md", "improve my workflow", "design tools", "build an agent", "context engineering", or "autonomous research".
---

# Claude Code Mastery

Build better agents, skills, and workflows with Claude Code. Battle-tested patterns from building Claude Code itself, used daily at Anthropic with hundreds of skills in production.

## The Three Laws of Agent Design

Before anything else, internalize these:

1. **The filesystem is how agents think.** Write to disk, grep, process. Don't stuff 100 items into context.
2. **Prompt caching is architecture, not optimization.** Design your entire system around prefix stability.
3. **Give Claude a way to verify its work.** This alone 2-3x the quality of output.

## When Building an Agent

### Action Space Design

> *Consult [tool-design reference](references/tool-design.md) for the complete tool design framework.*

Most agent failures are **tool design problems**, not model problems. Design tools by imagining yourself solving the problem:

- **Paper** = minimal (just text output). Limited but safe.
- **Calculator** = specific tools (custom tool_use). More capable but rigid.
- **Computer** = bash + filesystem. Most powerful, most flexible.

**Start with bash.** Claude Code started with 4 tools: read, write, edit, bash. That covers 80% of tasks. Add custom tools only when bash genuinely can't do the job.

**Anti-pattern:** 50 custom tools, one for each operation. This creates a "needle in haystack" problem — the model's reasoning degrades as tool count increases.

**The right question:** "What permissions and environment should I provide?" — not "What should I ask?"

### The Agent Loop

Every agent follows three phases:

1. **Gather Context** — Use agentic search (grep, find, bash) before semantic search. Let Claude build its own context through progressive disclosure.
2. **Take Action** — Bash for flexible operations, custom tools for frequently-used primaries, MCP for external services.
3. **Verify Work** — Explicit rules with feedback (linting), visual feedback (screenshots), LLM-as-judge for fuzzy evaluation.

> *Consult [agent-loop reference](references/agent-loop.md) for detailed patterns per phase.*

### Progressive Disclosure — Don't Load Everything Upfront

Agents get dumber when you give them too much information upfront.

At startup, load only skill names and descriptions. Let Claude discover details when needed. This applies to:

- Skills (name + trigger only, full content on demand)
- MCP tools (stubs with `defer_loading: true`, full schema via ToolSearch)
- Reference files (agent reads them when it decides to)
- Data (write to files, grep when needed)

## When Writing Skills

### Skill Categories

> *Consult [skill-categories reference](references/skill-categories.md) for all 9 categories with examples.*

The key categories:
1. **Library & API Reference** — How to use a library/CLI. Focus on gotchas, not obvious docs.
2. **Product Verification** — How to test and verify output. Worth a week of engineering.
3. **Data Fetching & Analysis** — Connect to data stacks with credentials and common queries.
4. **Business Process Automation** — Repetitive workflows as one command.
5. **Code Scaffolding** — Generate framework boilerplate with natural language requirements.
6. **Code Quality & Review** — Enforce org standards. Run as hooks or in CI.
7. **CI/CD & Deployment** — Fetch, push, deploy with testing and rollback.
8. **Runbooks** — Symptom -> investigation -> structured report.
9. **Infrastructure Operations** — Routine maintenance with safety guardrails.

### Skill Structure

A skill is a **folder**, not a file:

```
skills/skill-name/
  SKILL.md              # Instructions + trigger conditions
  references/           # Detailed docs Claude reads on demand
    api.md              # Function signatures, usage examples
    gotchas.md          # Known failure points (THE highest-signal content)
    examples.md         # Good and bad output samples
  scripts/              # Helper bash/Node scripts
    verify.sh           # Verify output has required sections
    fetch.sh            # Pre-built data fetching
  assets/               # Templates, configs
    template.md         # Output template to copy
  config.json           # Skill metadata, setup data
```

### The Description Field Is a Trigger, Not a Summary

**Bad:** `"Generates meeting summaries"`
**Good:** `"Use when a new meeting is detected or user asks to summarize, recap, or analyze a specific meeting. Relevant when user says 'what happened in', 'summarize the', 'meeting notes for', 'action items from'."`

The description is what Claude scans to decide if this skill applies. Write it as triggering conditions.

### Build a Gotchas Section

The highest-signal content in any skill is the Gotchas section.

Start small. Every time Claude fails at a task, add the failure mode:

```markdown
## Gotchas
- API returns empty results for queries under 3 characters
- Date field is UTC, not local timezone
- Rate limit is 100/min, batch requests in groups of 50
- Transcript can be null even when notes exist
```

This is how skills get better over time. The gotchas section is a living document.

### Don't Over-Specify Steps

**Anti-pattern:**
```
Step 1: Call API with query
Step 2: Take first 3 results
Step 3: Format as bullets
Step 4: Post to Slack
```

**Correct pattern:**
```
Search for relevant data, get details for the most relevant items,
and synthesize a clear answer. Reference sources by name and date.

Gotchas:
- Results capped at 50, paginate for exhaustive search
- Null fields are common, check before including
```

Tell Claude WHAT, not HOW. It's smart enough to figure out the steps.

### Include Helper Scripts

Don't make Claude reconstruct boilerplate each time. Provide scripts it can compose:

```bash
# scripts/fetch-data.sh — Claude calls this via bash
#!/bin/bash
curl -s "$API_URL/search?q=$1" | jq '.results[:10]'
```

Claude spends tokens on composition and reasoning, not on rebuilding the fetch logic.

### Store Execution History

```
data/standups.log        # Append-only log of every standup generated
data/last-digest.json    # State from last run
```

Claude reads its own history and can tell what's changed since the last run. Use `${CLAUDE_PLUGIN_DATA}` for durable storage across skill upgrades.

## When Structuring CLAUDE.md

A team should share a single CLAUDE.md, checked into git, updated multiple times a week. Anytime Claude does something incorrectly, add it.

> *Consult [claude-md reference](references/claude-md.md) for the complete structure guide.*

### The Rules

- **< 200 lines.** One engineer had 847 lines and got worse results than a 100-line version. Claude ignores bloated context.
- **Update on every mistake.** End your prompt: "Update CLAUDE.md so this doesn't happen again."
- **Treat it like code.** Review changes, test behavior, track in git.
- **Focus on what challenges Claude's defaults.** Don't document what Claude already knows.

### Recommended Sections

```markdown
# Project Name

## What This Is
One paragraph. What, why, for whom.

## Tech Stack
Languages, frameworks, key dependencies.

## Build & Test
Commands to build, test, lint, deploy.

## Architecture
Key directories, entry points, data flow.

## Conventions
Naming, patterns, anti-patterns specific to this project.

## Gotchas
Things Claude gets wrong. Updated continuously.
```

## When Optimizing Prompt Caching

At Anthropic, prompt caching is treated as critical infrastructure — alerts on cache hit rate, SEVs when they drop too low.

> *Consult [prompt-caching reference](references/prompt-caching.md) for the complete technical guide.*

### The Ordering Rule

Static content first, dynamic content last:

```
1. System prompt + tool definitions    (globally cached)
2. CLAUDE.md / project context         (cached per project)
3. Session context / MEMORY.md         (cached per session)
4. Conversation messages               (new each turn)
```

### The Five Commandments

1. **Never change tools mid-session.** Adding/removing a tool invalidates cache for the ENTIRE conversation.
2. **Never change models mid-session.** Caches are per-model. Switching to Haiku at 100K tokens costs more than letting Opus answer.
3. **Use messages for state updates.** Don't edit the system prompt. Add `<system-reminder>` in the next message.
4. **Defer tools instead of removing them.** Send lightweight stubs, let ToolSearch load full schemas on demand.
5. **Fork operations share the parent prefix.** Compaction uses the same system prompt, tools, and history prefix.

### State Transitions Are Tools, Not Config Changes

**Anti-pattern:** Enter plan mode by swapping the tool set.
**Correct:** `EnterPlanMode` is a tool. Agent calls it. Tools never change. Cache stays warm.

## When Using Subagents

Append "use subagents" to any request where you want more compute.

> *Consult [subagents reference](references/subagents.md) for coordination patterns.*

### When to Use

- Task needs a different model (Haiku for cheap lookups)
- Task produces lots of output that would pollute main context
- Multiple independent tasks can run in parallel
- Task is risky and you want to isolate it

### When NOT to Use

- Simple questions (one search + one answer)
- Sequential work depending on previous steps
- Cache switching cost > benefit

### Production Subagent Patterns

- `code-simplifier` — Post-implementation cleanup
- `verify-app` — End-to-end testing
- `adversarial-review` — Fresh-eyes subagent critiques code, iterates until findings degrade to nitpicks

## When Setting Up Verification

Probably the most important thing to get great results — give Claude a way to verify its work. 2-3x quality improvement.

> *Consult [verification reference](references/verification.md) for patterns and examples.*

### Verification Patterns

| What you're doing | How to verify |
|---|---|
| Writing code | Run tests, linter, type checker |
| Creating files | Read them back, check required fields |
| UI changes | Open browser (Playwright), screenshot, iterate |
| API calls | Check response status, validate schema |
| Data processing | Compare input/output counts, spot-check values |
| Meeting summaries | Run verify.sh to check required sections exist |

### The Verification Script Pattern

```bash
# scripts/verify.sh — Claude runs this after generating output
#!/bin/bash
FILE="$1"
ERRORS=0
grep -q "## Overview" "$FILE" || { echo "MISSING: Overview"; ERRORS=$((ERRORS+1)); }
grep -q "## Action Items" "$FILE" || { echo "MISSING: Action Items"; ERRORS=$((ERRORS+1)); }
[ "$ERRORS" -gt 0 ] && exit 1 || echo "PASS"
```

## When Working in Parallel

Spin up 3-5 git worktrees, each running its own Claude session. This is the single biggest productivity unlock — 20-30 PRs per day from parallel sessions.

> *Consult [parallel-work reference](references/parallel-work.md) for worktree and team patterns.*

### The Pattern

```bash
# Create worktrees for parallel work
claude -w feature-auth &
claude -w feature-search &
claude -w fix-bug-123 &
```

Each gets its own branch, working directory, and context. No file conflicts.

### Recommended Setup

- 5 terminal tabs (numbered 1-5)
- Shell aliases to hop between worktrees with one keystroke
- System notifications when Claude needs input
- 5-10 concurrent web sessions on claude.ai/code
- Mobile sessions checked throughout the day

## When Building Non-Coding Agents

Claude Code's power comes from bash + filesystem access, not from "being a coding tool." Use the bash tool more.

> *Consult [non-coding-agents reference](references/non-coding-agents.md) for patterns.*

Claude Code works for non-coding tasks: data analysis, email processing, file management, web research, Excel/CSV work.

### The Autonomous Research Pattern

For agents that iterate autonomously:

1. **Single file to edit** — the agent's workspace
2. **Single metric to optimize** — testable, objective
3. **Fixed time budget** — each iteration is bounded
4. **Reversibility** — changes can be reverted if results don't improve
5. **Instructions in markdown** — humans steer via `program.md`, agents act via code

> *Consult [autonomous-research reference](references/autonomous-research.md) for the full autonomous research framework.*

### Context Engineering

Context engineering is the art and science of filling the context window with just the right information for the next step.

Don't just optimize the prompt. Optimize everything the agent sees: memory, examples, tools, state, control flow. The whole context window is your UI to the agent.
