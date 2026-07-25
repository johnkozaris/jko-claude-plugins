# Lifecycle and navigation

Use this reference for repeated work, disappearing tasks, reset state, unstable
lists, sheets, deep links, or tab navigation.

SwiftUI view values are recreated frequently. Keep side effects out of `init`
and rendering expressions. Use lifecycle-aware tasks for asynchronous work that
should cancel with the view. Use appearance callbacks only when their repeat
semantics are acceptable.

Identity determines whether state survives. Inspect conditional branches,
`ForEach` IDs, explicit `.id`, and extracted subviews before blaming
Observation. Random or index-based IDs can reset state, replay transitions, and
destroy list performance.

`@State` seeded from a parameter initializes once per identity and does not
track later parameter changes. Pass changing values down directly or key the
view by the intended identity.

Model navigation as state when the product needs deep links, restoration,
programmatic routing, or independent tab histories. Typed destinations reduce
string drift. Give independent tabs independent histories. A short linear flow
does not need an application-wide router merely because one is possible.

Drive optional destinations and sheets from the data they present when that
removes contradictory booleans. Validate dismissal, restoration, malformed
deep links, and schema changes rather than assuming a bound path provides those
behaviors automatically.
