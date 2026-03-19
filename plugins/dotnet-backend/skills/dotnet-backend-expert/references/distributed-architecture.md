# Distributed Architecture

## Start With a Monolith That Has Boundaries

A modular monolith beats a premature microservice fleet most of the time.

Stay monolithic when you need:

- strong consistency
- fast local debugging
- low latency between workflows
- simple deployment and incident response
- shared data that is not yet clearly partitioned

## Extraction Tests

Split a service only if you can say yes to most of these:

1. one stable business capability owns it
2. one team or owner can operate it
3. it can own its own data without cross-service joins
4. it benefits from independent deploy or scale
5. eventual consistency is acceptable where boundaries cross
6. observability, retry, versioning, and incident handling are accounted for

## Messaging

Use messaging when asynchronous decoupling solves a real problem:

- workflow fan-out
- integration across service boundaries
- load leveling
- retry isolation
- event propagation to independent consumers

Do not use messaging to hide unclear boundaries.

## Outbox and Idempotency

If a write and an event must succeed together:

- use a transactional outbox or equivalent reliable pattern
- make consumers idempotent
- design for duplicate or delayed delivery
- preserve ordering where the business depends on it

Avoid dual-write optimism.

## Signs Distributed Architecture Is Overkill

- same user flow requires many synchronous service hops
- correctness depends on cross-service transactions
- the split exists only to make folders feel cleaner
- the team cannot explain retry, dedupe, tracing, and failure handling
- a queue or broker appears before the bounded context is clear

## Review Questions

- What is the source of truth for this business write?
- What happens if a message is duplicated, delayed, or replayed?
- Does independent deployment actually matter here?
- If we removed the broker or extra service, what real capability would we lose?
