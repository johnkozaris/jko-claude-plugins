# Accessibility selectors

The compact snapshot prints role, accessible name, value, heading level,
checked/pressed/selected/expanded state, focus, readonly, and disabled state
when Chromium exposes them.

Prefer selectors in this order:

1. role plus accessible name;
2. exact visible text;
3. stable test ID;
4. scoped CSS when semantics are unavailable.

Standard ARIA tabs normally expose `selected=true`; `pressed` describes toggle
buttons. Verify the snapshot rather than assuming one attribute.

Playwright text selectors differ by quoting: unquoted `text=Save` is a
case-insensitive substring; `text="Save"` is an exact case-sensitive match.

## Strict-mode failures

Playwright actions require one target. A strict-mode violation means the
selector matched multiple nodes. Do not choose one arbitrarily. Scope through a
semantic parent with `>>`, use an exact name, or use an explicit `nth=` only
after the current snapshot proves the ordering is meaningful.

Examples of shape, not app-specific names:

```text
role=navigation >> role=button[name="Settings"]
role=dialog >> role=button[name="Confirm"]
text="Save"
[data-testid=primary-action]
```

Canvas/WebGL/xterm content may not appear in the accessibility tree. Verify the
container structurally and use bounded pixel evidence or a read-only
application-specific inspection API when necessary.
