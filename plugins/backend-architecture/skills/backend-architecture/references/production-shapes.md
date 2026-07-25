# Production architecture shapes

## Modular monolith

Prevent direct cross-module storage access and writes that bypass the owning
module; expose deliberate in-process contracts for collaboration.

## Independently deployed services

Require explicit timeout and retry budgets, per-dependency concurrency limits,
idempotency, compatibility, telemetry, and degraded behavior. Shrink timeout
budgets downstream rather than allowing every hop to retry independently.

## Event-driven flows

Use an outbox or equivalent atomic publication boundary when database state and
event publication must not diverge. Do not use events to hide an unclear owner.

When a transaction crosses owners, make each step and compensation explicit and
give coordination a clear owner. Use orchestration when the sequence must be
observed or changed in one place; use choreography when steps are genuinely
independent. Durable execution can buy retries, timers, and visibility at the
cost of another runtime.

## Functional core, imperative shell

Useful when deterministic business rules are surrounded by databases, clocks,
networks, or queues. Keep decisions in pure transformations and effects in a
thin shell. This is often enough architecture for a small service and avoids
mock-heavy layering.

## Actor or owned-state component

Useful for long-lived concurrent state with serialized mutation: sessions,
devices, rooms, schedulers, or connection managers. Define mailbox bounds,
supervision, persistence, shutdown, and behavior when an owner is unavailable.
Do not turn every entity into an actor.

## Pipeline

Useful for staged parsing, media/data processing, ETL, or protocol handling.
Make ownership transfer, backpressure, cancellation, partial failure, and
restart points explicit. A pipeline without bounded stages only moves the
resource leak.

## Extraction sequence

Extract behavior before infrastructure:

1. Enforce the intended module boundary in-process.
2. Add contract tests around the future seam.
3. Assign schema and write ownership.
4. Use expand/contract changes for shared data.
5. Backfill while writes remain compatible; shadow-read to compare results.
6. Shift reads and writes in the order supported by the chosen authority,
   replication direction, consistency requirements, and rollback mechanism.
   Do not cut a path until freshness and reconciliation are verified.
7. Separate deployment only after data ownership and failure behavior are real.

The data migration and consistency window usually set the timeline, not moving
classes into another process. Avoid indefinite dual writes; define the
authority, reconciliation, and removal milestone before introducing them.

## Decision test

For any proposed shape, ask:

1. What state does it own?
2. What changes together?
3. What is the consistency boundary?
4. How does it fail and recover?
5. How is load bounded?
6. How is behavior observed?
7. What becomes independently deployable or testable?
8. Which concrete problem is simpler than before?

If those answers are vague, keep the architecture simpler and gather evidence.
