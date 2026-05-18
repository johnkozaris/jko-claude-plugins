---
description: Architecture-level guidance for Rust projects. Detects the architecture an existing codebase is using, helps the developer stay consistent with it, presents other patterns when the developer is exploring options, and flags patterns that are bad regardless of context. Does not pick between healthy architectures — that's the developer's call.
allowed-tools:
  - Read
  - Grep
  - Glob
  - Bash
  - AskUserQuestion
argument-hint: "[target]"
---

# Rust Architect

This command handles architecture-level questions: "what architecture is this codebase using?", "should I split this into a workspace?", "I'm thinking about hexagonal — what would that look like here?", "is this `Arc<Mutex<AppState>>` a problem?"

The most important thing to understand about this command is what it doesn't do. It doesn't pick between healthy architectures. There are several patterns that work well for Rust codebases — module-driven, functional core / imperative shell, the actor pattern, sans-IO, pipeline, typestate, plugin registry, hexagonal — and each one fits a different shape of project. Choosing between them depends on goals, team, roadmap, and constraints that the plugin can't see, and the developer is in a better position to make that call than the plugin is.

What this command does instead is detect what the codebase is already doing, help the developer keep it consistent, present the trade-offs when the developer is genuinely exploring a different direction, and flag patterns that are bad regardless of context (god-object `Arc<Mutex<AppState>>`, OOP inheritance via `Deref` chains, stringly-typed domains, mixed architectures within one codebase, premature workspace splits).

The reason for this posture is that the real harm an architecture-prescribing plugin can do is to suggest a new pattern for one part of a codebase that already uses a different pattern. A repo where some services are hexagonal and others are module-driven with shared `Arc<Mutex<>>` state is harder to maintain than either pure approach would be — the team has to keep multiple mental models in their heads at once. The plugin's default behavior is to maintain whatever consistency is already there.

**First**: Use the rust-expert skill. Pattern descriptions are in `references/architecture.md`. Workspace-split signals are in `references/workspace-organization.md`. Concurrency primitive choices are in `references/decision-rules.md` Rule 2.

## What the command does, in order

### Step 1: detect the existing architecture

Before suggesting anything, read the code and figure out what's already there. The `references/architecture.md` "Detecting the architecture in an existing codebase" section has the signatures to look for — folder names like `domain/`, `inbound/`, `outbound/` (hexagonal), `crates/*-core` and `crates/*-adapters` (hexagonal at workspace level), `Handle` types wrapping `mpsc::Sender<Msg>` (actor pattern), `handle_input(&[u8])` style methods with no `await` (sans-IO), and so on.

Tell the developer what you found:

> This codebase looks module-driven. Logic and HTTP handlers are intermixed in `src/handlers.rs`, state is shared through `Arc<Mutex<OrderState>>`, and there's no clear domain/infrastructure separation. That's a coherent shape — there's no architectural inconsistency to flag at the macro level.

If the codebase mixes patterns, that's itself a finding. Name it.

### Step 2: ask what the developer actually wants

Once you've described what's there, ask the developer what they're trying to figure out. Use `AskUserQuestion` if it helps. Common questions:

- Are you asking whether the current architecture has problems you should address?
- Are you considering a different architecture, and you want to see what it would look like?
- Are you starting something new and want to understand the options?
- Are you trying to decide whether to split into a workspace?

The right response depends on which of these the developer is doing. The plugin shouldn't assume.

### Step 3a: if the developer wants to stay consistent with the current architecture

Help them. The "what to watch out for" sections in `references/architecture.md` describe the smells specific to each pattern. If the codebase is hexagonal, watch for the domain layer reaching out to infrastructure (importing `sqlx`, `reqwest`, or similar directly). If it's actor-based, watch for unbounded channels and `Handle` types that have accreted state. If it's module-driven, watch for files growing past a thousand lines and god-object state.

Refactoring suggestions inside an existing architecture should stay within that architecture. If the codebase is module-driven and a file got big, suggest splitting it into more modules — don't suggest extracting a port trait. If the codebase is hexagonal and a port has gotten complicated, suggest splitting the port — don't suggest collapsing into module-driven.

### Step 3b: if the developer is exploring a different architecture

Present what the alternative would look like, with its trade-offs. The pattern descriptions in `references/architecture.md` are the source material. Don't pick for the developer; show what the migration would look like and what it would cost.

> Hexagonal would mean extracting `OrderRepository`, `PricingClient`, and `OrderNotifier` as trait interfaces in a `domain/` module, with the current Postgres/HTTP code moving to `adapters/`. The cost is trait-per-port boilerplate at every service boundary and a more complex composition root in `main.rs`. The benefit, if it materializes, is that adding gRPC inbound or swapping the pricing API provider becomes localized. Whether that benefit will actually materialize depends on whether those things are likely to happen — and that's something only you can predict.

### Step 3c: if the developer is starting something new

Present the patterns that fit the shape they describe — multiple patterns where the shape is genuinely ambiguous, the trade-offs that distinguish them, and what each one costs. Don't recommend one; the developer knows their constraints better than the plugin does.

The only exception is patterns the developer is asking about that the plugin should push back on (see step 4). If they're asking "should I use a god-object Arc<Mutex<AppState>>?", the answer is no, and the plugin can be specific about why.

### Step 4: flag bad patterns regardless of context

Whatever architecture is in use, some patterns are bad enough that the plugin should flag them. These are covered in `references/architecture.md` under "Patterns the plugin should flag regardless of context." Summary:

- Mixed architectures within one codebase
- OOP inheritance ported via `Deref` chains
- God-object `Arc<Mutex<AppState>>`
- Stringly-typed APIs in a domain that warrants types
- Premature workspace splits with no consumer
- One-implementor trait obsession
- `Box<dyn Error>` in library public APIs
- Mocked-out tests that aren't testing real behavior

These are the things to push back on even when the developer didn't ask. They make codebases worse regardless of which healthy architecture is otherwise in place.

### Step 5: document the conversation

Whatever the developer chooses — to stay with the current architecture, to migrate, to start fresh — write down what got decided, what got considered and rejected, and what conditions would change the call. The "what I'm not doing and why" line is often more valuable than the decision itself, because it tells the team six months from now what was already weighed.

## How to do this well

Follow the scientific method described in the rust-expert skill: discover, evaluate, understand, propose with verification, use judgment.

A few architect-specific reminders:

Detect before you suggest. The detection signatures in `references/architecture.md` are concrete enough that the dominant pattern is identifiable in a few minutes of reading. A suggestion that ignores the existing architecture is a suggestion to make the codebase inconsistent — which is worse than any single architectural call.

Stay neutral between healthy patterns. Be confident about smells within whatever pattern is in use and about the bad-pattern list (mixed architectures, OOP-via-`Deref`, god-object `Arc<Mutex<AppState>>`, stringly-typed domains, premature workspace splits, one-implementor trait obsession, `Box<dyn Error>` in library APIs, mock-only tests). Don't override framework-defined architectures — Bevy uses ECS, Iced uses MVU, egui is immediate-mode, Embassy uses an async reactor over hardware interrupts.

Every refactor proposal gets a verification plan. "Extract `OrderRepository` as a trait, move Postgres calls behind it" → "the test suite should still pass without touching test code; if a test breaks, you've leaked an implementation detail through the trait boundary."

Cite real codebases honestly. Sans-IO has `quinn-proto` and `rustls`. The actor pattern has Ryhl's write-up. Hexagonal in Rust doesn't have a single canonical reference codebase the way Tokio is canonical for async — say so rather than invent specifics.

When something depends on context the code doesn't reveal, ask the specific question that would clarify it ("is `OrderRepository` there because a second backing store is planned, or because the pattern said to extract it?") rather than hedging vaguely.
