# Tool Design — The Action Space Framework

> Source: Lessons from building Claude Code

## The Core Problem

Most agent failures in production aren't model problems — they're tool design problems. The difference between a working agent and a broken one comes down to how you structure the agent's "action space" — the universe of choices the model can make.

## The Calculator Analogy

Imagine being given a hard math problem. What tools would you want?

| Tool | Capability | Limitation |
|---|---|---|
| **Paper** | Minimal. Manual calculations. | Slow, error-prone |
| **Calculator** | Specific operations. Fast. | Must know which buttons to press |
| **Computer** | Write and execute code. | Must know how to program |

Design your agent's tools to match its abilities. Claude is a computer-level reasoner — give it computer-level tools (bash, filesystem).

## The Tool Explosion Problem

MCP servers commonly expose 50+ tools each. Users with 7+ servers = 67,000+ tokens just for tool definitions. In a 200K context window, that's 1/3 of budget gone before the user types anything.

**The model's reasoning degrades** as tool count increases — it's a "needle in haystack" problem.

## Solutions

### 1. Minimal Tool Set (Default)
Claude Code uses 4 tools: read, write, edit, bash. This handles 80% of all tasks.

### 2. Dynamic Tool Loading
When tool descriptions consume >10% of context, switch to search-based loading:
- Send lightweight stubs (name only, `defer_loading: true`)
- Agent discovers full schemas via ToolSearch when needed
- Cached prefix stays stable

### 3. MCP Server Instructions
The `instructions` field in MCP server config is metadata for tool discovery. Write it as "when to use this server", not "what this server does."

## Tool Design Checklist

- Does this need to be a tool, or can bash do it?
- Is the tool name self-explanatory?
- Does the description explain WHEN to use it (trigger), not just WHAT it does?
- Are parameters minimal (only what's needed)?
- Does the tool return structured output (JSON preferred)?
- Is the output truncated to avoid context overflow (<50KB)?
- Does the tool include error messages that help the model recover?

## The AskUserQuestion Evolution

How the Claude Code team iterated on the elicitation tool:

1. **Attempt 1:** Added questions array to ExitPlanTool → confused Claude (simultaneous plan + questions)
2. **Attempt 2:** Modified output format to markdown with bracketed options → not guaranteed (Claude appended extra text)
3. **Attempt 3:** Dedicated `AskUserQuestion` tool with structured parameters → Claude liked calling it, outputs worked well

**Lesson:** Even the best-designed tool doesn't work if Claude doesn't understand how to call it. Test with the model.

## The Todo → Tasks Evolution

As models improved:
- Todo list + system reminders every 5 turns → Claude thought it had to stick to the list
- Replaced with Task tool → tasks support dependencies, shared updates across subagents, can be altered/deleted
- **Lesson:** What worked for one model generation may constrain the next. Revisit tool design as capabilities increase.
