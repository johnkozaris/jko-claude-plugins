# State and architecture

Use this reference when ownership, Observation, ViewModels, or module boundaries
are the actual decision.

`@Observable` invalidation is dependency-scoped: SwiftUI records key-path reads
while evaluating a view rather than treating every mutation as invalidating
every consumer. Tracking registers only for properties read during that
evaluation. A property read only in `onAppear`, inside a task or closure, or
through a non-observable intermediary will not invalidate the view.

Default to SwiftUI's native data flow. A view-local value belongs in `@State`;
shared observable state needs one clear ancestor owner; a received model should
not be re-owned accidentally. Use `@Bindable` to derive bindings from an
observable instance rather than adding a wrapper object solely for `$` access.

A ViewModel earns its existence by owning orchestration that remains meaningful
outside the view: a state machine, retries, pagination, optimistic updates,
independent lifecycle, or a valuable test seam. Moving every property and
button action into a class is not architecture.

Adopt TCA or another state framework only when cross-feature coordination,
deterministic effect handling, or team-wide consistency repays its dependency
and ceremony. Preserve an established coherent architecture unless the current
problem demonstrates its cost.

`@AppStorage` and `@SceneStorage` are `DynamicProperty` types whose update
mechanism runs inside a `View`, `Scene`, or `App`. Placing one in a plain class,
including an `@Observable` class, can provide storage without SwiftUI
invalidation. Verify the behavior with a focused test instead of assuming the
Observation macro adapts the wrapper.
