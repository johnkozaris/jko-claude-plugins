# Swift concurrency in app targets

Inspect the project's Swift version, strict-concurrency mode, default isolation,
and target type before recommending annotations.

Main-actor defaults often fit application UI targets because observable UI
state is main-thread-owned. Reusable parsing, networking, and domain packages
may need nonisolated APIs so callers can use them without unnecessary actor
hops. Do not spread one target's isolation policy across every package.

Prefer structured tasks whose lifetime and cancellation are owned by the caller
or view. Detached and fire-and-forget work require an explicit reason, error
path, and shutdown story.

Treat `@unchecked Sendable` as a synchronization assertion. The code should make
the mechanism visible: immutability, actor isolation, a lock, or another
well-defined owner. Adding the conformance to silence a compiler diagnostic
moves a race from compile time to production.

When bridging delegates, callbacks, Objective-C, or C APIs, identify the thread
and lifetime contract at that boundary. Verify current SDK signatures and
compiler behavior instead of copying annotations from a different toolchain.
