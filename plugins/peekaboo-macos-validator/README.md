# Peekaboo macOS Validator

Runtime automation and visual verification for native SwiftUI and AppKit apps.

Use Peekaboo when the unresolved part of a task lives in the running interface:
rendering, accessibility state, focus, selection, menus, windows, dialogs, or a
visible workflow.

When UI evidence is required, the plugin prefers accessibility state and
semantic actions. Screenshots are captured only when pixels answer a real
question. When the host supports isolated visual work, artifacts stay in a
separate reader until it returns a compact report; the reader is then
terminated and verified gone. Every screenshot is treated as large, likely-large
JSON is filtered from disk, and temporary artifacts are deleted after use.

Requires `peekaboo` and `jq` on macOS.

The primary `peekaboo` skill includes setup diagnosis. Use the additional
`validate-macos-app` skill for a state-preserving application validation flow.
