# Testing and API currency

Use current project and SDK evidence before recommending an API replacement.
An older API that is supported, coherent, and outside the change is not a
finding merely because a newer spelling exists.

Look up availability and deprecation in current Apple documentation or SDK
interfaces. Distinguish compiler deprecation from community preference and
future-looking marketing.

Test observable behavior:

- state ownership and mutation causing the expected view update;
- task cancellation when identity or navigation changes;
- route parsing and restoration failures;
- persistence migration from a previous store;
- accessibility output and keyboard behavior;
- the user-visible regression that motivated the change.

Previews are useful executable examples, especially for empty, long-text, and
error states, but they do not replace tests or runtime validation. Use Swift
Testing or XCTest according to the project's existing stack and the capability
being tested.

Use structured logging with appropriate privacy. Keep debug-only diagnostics
out of production behavior and avoid logging secrets or personal data.
