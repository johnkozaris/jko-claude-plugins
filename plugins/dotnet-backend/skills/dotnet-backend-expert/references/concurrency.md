# Concurrency & Async

## Async All the Way

Backend I/O should stay async from the host boundary through external calls.

**Do**:

- propagate `CancellationToken`
- use truly async I/O APIs
- keep `Task`/`Task<T>` as the default async shape
- use `ValueTask` only with measured benefit

**Don't**:

- call `.Result`, `.Wait()`, or `GetAwaiter().GetResult()` in request or worker paths
- wrap I/O in `Task.Run()` to simulate async

## Shared State

Default to immutable shared state. If state must be shared:

- make ownership explicit
- use thread-safe collections when multi-writer access is real
- guard multi-step invariants carefully

A mutable singleton is a concurrency design decision, not a convenience.

## Coordination Tools

### `Channel<T>`

Use for producer/consumer work and background pipelines. Prefer bounded channels when backpressure matters.

### `SemaphoreSlim`

Use to cap concurrency for async work. Always release in `finally`.

### `lock`

Use only for short synchronous critical sections. Never `await` inside a lock.

## Background Work

Hosted services should:

- respect cancellation and shutdown
- create scopes for scoped dependencies
- coordinate work, not own the domain rules
- avoid ad hoc in-memory queue implementations when a channel fits

## Fire-and-Forget

Assume fire-and-forget is a bug until proven otherwise.

If work matters, give it an owner:

- queue it
- track it
- log it
- handle shutdown semantics

## Concurrency Smells

| Smell | Signal | Fix |
|---|---|---|
| Sync-over-async | `.Result`, `.Wait()`, blocking wrappers | make the whole path async |
| Hidden shared state | static or singleton mutable collections | isolate ownership or synchronize properly |
| Ad hoc queue | list + lock + polling loop | use `Channel<T>` |
| Scoped-in-worker bug | hosted service uses scoped dependency directly | create a scope or use factory |
| `await` in `lock` | impossible or fragile critical section | redesign with async-safe coordination |
