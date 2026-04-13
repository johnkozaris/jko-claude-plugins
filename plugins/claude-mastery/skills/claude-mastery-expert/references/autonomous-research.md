# Autonomous Research — Iterative Agent Loops and Context Engineering

## The Autonomous Research Loop

630 lines of Python. An agent modifies code, trains for 5 minutes, checks if results improved, keeps improvements, discards failures, repeats.

**Results:** 126 experiments overnight. Loss improved from 0.9979 to 0.9697. 700 experiments over 2 days found 20 genuine optimizations and an 11% speedup on already-optimized code.

### The Three-Component Loop

1. **Single file to edit** — the agent's entire workspace fits in one context window
2. **Single metric to optimize** — testable, objective, no ambiguity
3. **Fixed time budget** — each iteration bounded (5 minutes)

### The Three-File Architecture

```
prepare.py    # Fixed constants, one-time data prep. NEVER modified by agent.
train.py      # THE single file the agent edits. All experimentation here.
program.md    # Research instructions in plain English. Human steers, agent acts.
```

### Key Technical Patterns

**Single Metric, Not Vague Goals:**
Replace "improve performance" with a concrete, testable number. Single metrics force focus and enable automated evaluation.

**Reversibility:**
Every change the agent makes can be reverted if results don't improve. Failure is data. The agent learns from reversion.

**Instructions in Markdown:**
`program.md` contains what the agent SHOULD do, SHOULD NOT do, when to stop, and constraints on the search space. This is plain English, not code configuration.

**Logging and Introspection:**
Every experiment produces a log: changes made, metrics before/after, whether kept or reverted. Humans read the full history of agent reasoning.

**Grid Search with Parallelism:**
Single GPU: greedy hill-climbing. 16 GPUs: factorial grids of 10-13 experiments per wave, catching interaction effects sequential search would miss.

## Context Engineering

Context engineering is the art and science of filling the context window with just the right information for the next step.

This goes beyond prompt engineering. It covers:
- Examples and few-shot patterns
- Memory systems (what to remember, what to forget)
- Retrieval (when to search, what to search for)
- Tools and their descriptions (action space design)
- State representation (how to show the world to the agent)
- Control flow (when to stop, when to branch, when to retry)
- Reasoning traces (chain of thought, scratchpads)

**The 630-line constraint** in the autonomous research loop is a direct application: keep everything the agent needs in one cohesive unit within the context window.

## Agentic Engineering

Evolution from "vibe coding" (expressing intent, AI generates code) to "agentic engineering" (orchestrating agents who do the work, acting as oversight).

**Components:**
- Multi-agent coordination (implementation, testing, security as separate agents)
- Automated verification at every step
- Bounded autonomy (clear success criteria, time limits)
- Human evaluation before production merge
- Audit trails and governance

**The key shift:** You're the architect and supervisor, not the implementer.

## The December 2025 Phase Shift

Models crossed a coherence threshold around December 2025. Agents can now:
- Work for 30+ minutes on complex tasks
- Research solutions online, resolve issues one by one
- Maintain long-term coherence across multi-step workflows

The shift went from 80% manual + 20% agent to 80% agent + 20% edits in about 3 months.

## The Decade of Agents

Don't expect full autonomy in 1-2 years. Five hard problems remain:
1. Continual learning (agents can't improve from experience)
2. Durable memory (short context, ephemeral state)
3. Multimodality with computer use (can't reliably see and click)
4. Reliable process-level supervision (hard to verify agent decisions)
5. Architectural diversity (preserving reasoning, stripping brittle memorization)

**Historical parallel:** Self-driving cars: perfect demo in 2013, still not fully solved in 2025. Don't confuse 6-month demos with decade-long product cycles.

## Actionable Takeaways

1. **Meet LLMs halfway** — Design your system for agents, not humans navigating human interfaces
2. **Simplicity as architecture** — If agents will read/modify your code, prioritize clarity over cleverness
3. **Bounded autonomy with clear contracts** — Success criteria, constraints, time limits, reversion capabilities
4. **Context engineering over prompt engineering** — Optimize the whole context window, not just the prompt
5. **Verification as first-class pattern** — Automated tests, human review gates, rollback, audit trails
6. **Orchestration, not code generation** — Your role: architect + supervisor. Agents: implementers.
