# Prompt Caching — The Architecture You Must Design Around

> Source: Lessons from building Claude Code

## Why This Matters

At Claude Code, prompt caching is treated as critical infrastructure:
- Alerts on cache hit rate drops
- SEVs declared when cache breaks happen
- Every feature decision considers cache impact
- A few percentage points of cache miss dramatically affects cost and latency

## How It Works

Prompt caching uses **prefix matching**. The API caches everything from the start of the request up to each `cache_control` breakpoint. The order you put things in matters enormously.

## The Ordering Rule

```
Most stable (top)     ← cached across all sessions
  1. Static system prompt
  2. Tool definitions
  3. CLAUDE.md / project context
  4. Session context (MEMORY.md)
  5. Conversation messages
Most dynamic (bottom) ← new each turn
```

This maximizes how many sessions share cache hits.

## What Breaks the Cache (Don't Do These)

| Action | Why it breaks cache | Fix |
|---|---|---|
| Put timestamp in system prompt | Changes every request | Use `<system-reminder>` in messages |
| Shuffle tool definitions | Different prefix each time | Deterministic tool order |
| Add/remove tools mid-session | Invalidates entire prefix | Keep all tools, use defer_loading |
| Change model mid-session | Caches are per-model | Use subagents for model switching |
| Update tool parameters dynamically | Changes cached tool schemas | Use messages for dynamic state |
| Different system prompt for compaction | Rebuilds entire cache | Reuse parent's exact prefix |

## Use Messages for State Updates

**Anti-pattern:** Edit the system prompt to say "it is now Wednesday"
**Correct:** Add `<system-reminder>it is now Wednesday</system-reminder>` in the next user message

This preserves the cached prefix. All dynamic state goes in messages.

## Don't Change Models Mid-Session

At 100K tokens into an Opus conversation, switching to Haiku for a "simple question" means:
- Rebuilding the entire cache for Haiku (paying full price for 100K input tokens)
- Actually more expensive than just letting Opus answer

**Solution:** Use subagents. Opus prepares a "handoff" message to Haiku on the specific sub-task. Each model gets its own cache.

## Never Add or Remove Tools

Tools are part of the cached prefix. Adding or removing a tool mid-conversation invalidates the cache for everything after it.

**How Claude Code handles Plan Mode:**
- Intuitive approach: swap to read-only tools in plan mode → BREAKS CACHE
- Actual approach: `EnterPlanMode` is a tool. Agent calls it. System message explains the constraint. Tools never change.

**How Claude Code handles Tool Search:**
- Intuitive approach: remove unused MCP tools → BREAKS CACHE
- Actual approach: send lightweight stubs with `defer_loading: true`. Agent discovers full schemas via ToolSearch when needed. Stubs are always present in same order.

## Cache-Safe Compaction

When context fills up, conversation is summarized and a new session continues.

**Naive approach:** Separate API call with different system prompt and no tools → cached prefix doesn't match → full price for all input tokens.

**Claude Code's approach:** Compaction uses the exact same system prompt, user context, tools, and conversation history as the parent. The compaction instruction is appended as a new user message at the end. From the API's perspective, this looks nearly identical to the parent's last request.

Save a "compaction buffer" — room in the context window for the compact message + summary output tokens.

## Monitoring

Track these metrics in production:
- Cache hit rate (should be >90%)
- Cache miss events (log the cause)
- Cost per session (cache misses spike cost)
- TTFT (time to first token) — cache hits reduce this 5-10x
