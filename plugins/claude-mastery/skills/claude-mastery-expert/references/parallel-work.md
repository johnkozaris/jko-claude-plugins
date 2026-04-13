# Parallel Work — Worktrees, Teams, and tmux

> Source: Claude Code team practices, Anthropic official docs

## The Biggest Productivity Unlock

Spin up 3-5 git worktrees, each running its own Claude session. This is the single biggest productivity unlock — 20-30 PRs per day from parallel sessions.

## Git Worktrees

Each worktree gets its own branch and working directory. All share the same `.git` directory.

```bash
# Start Claude in its own worktree
claude -w feature-auth
claude -w feature-search
claude -w fix-bug-123

# With tmux for session management
claude -w feature-auth --tmux
```

**Why worktrees, not branches:** File conflicts vanish by design. Each agent assumes exclusive control over its working directory.

## Recommended Setup

- 5 terminal tabs numbered 1-5
- Shell aliases to hop between worktrees with one keystroke
- System notifications when Claude needs input
- 5-10 concurrent web sessions on claude.ai/code
- Mobile sessions checked throughout the day

## tmux Multi-Agent Pattern

```bash
# Create sessions
for task in auth search billing; do
  tmux new-session -d -s "$task" "claude -w $task"
done

# Send input to another agent
tmux send-keys -t auth -l "Fix the JWT validation bug"
sleep 0.3
tmux send-keys -t auth -H 0d  # Enter

# Read output
tmux capture-pane -t auth -p | tail -20
```

## Decomposition Strategy

Not all tasks parallelize. Good candidates:

| Parallelizable | NOT parallelizable |
|---|---|
| Independent features | Sequential data pipeline steps |
| Frontend + backend + tests | Changes to the same file |
| Multiple bug fixes in different modules | Refactoring that touches everything |
| Research + implementation + docs | Work that depends on previous work's output |

## Agent Teams (Experimental)

For coordinated parallel work:
- Team lead orchestrates
- Teammates communicate directly (not through lead)
- Shared task list for coordination
- Each teammate has its own context window

Enable: `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`

## Throttling

- Pro tier: 2-3 comfortable concurrent instances
- Max/Team tier: 5+ concurrent instances
- Modern Macs handle 3-5 sessions without issue
- Use notifications to know when sessions need input
