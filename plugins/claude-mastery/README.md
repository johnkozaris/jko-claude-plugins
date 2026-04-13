# claude-mastery

Master guide for building with Claude Code — distilled from the creators themselves.

## What's Included

### Skill
- `claude-mastery-expert` — Activated when building agents, writing skills, optimizing caching, designing tools, or improving workflows

### Commands
- `/mastery-audit` — Audit your project's Claude Code setup against best practices
- `/mastery-skill` — Guided skill creation following best practices
- `/mastery-teach` — Learn a specific concept with examples from the creators

### Reference Library
- `tool-design.md` — Action space design, tool count management, MCP tool search
- `prompt-caching.md` — Ordering rules, cache-breaking actions, compaction
- `verification.md` — The 2-3x quality multiplier, verification scripts, patterns
- `skill-categories.md` — All 9 skill categories with examples from Anthropic
- `claude-md.md` — Structure, sizing, update rituals, measuring effectiveness
- `subagents.md` — Coordination, model switching, production subagent patterns
- `parallel-work.md` — Worktrees, tmux, agent teams, decomposition strategy
- `autonomous-research.md` — autonomous research loops, context engineering, agentic engineering
- `agent-loop.md` — Gather/Act/Verify phases, progressive disclosure, filesystem as memory
- `non-coding-agents.md` — Email, data analysis, reports — Claude Code isn't just for code

## Install

```bash
# From Claude Code marketplace
/plugin install claude-mastery@johnkozaris/jko-claude-plugins

# Or manually
cp -r plugins/claude-mastery ~/.claude/plugins/claude-mastery
```

## Philosophy

Agent performance depends more on the system around the model than the model itself. This plugin helps you build better systems around Claude.
