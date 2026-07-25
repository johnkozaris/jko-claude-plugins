# Peekaboo macOS Validator

Runtime automation and visual verification for native SwiftUI and AppKit apps.

Use Peekaboo when the unresolved part of a task lives in the running interface:
rendering, accessibility state, focus, selection, menus, windows, dialogs, or a
visible workflow.

When UI evidence is required, the plugin prefers accessibility state and
semantic actions. Screenshots are captured only when pixels answer a real
question. When the host supports isolated visual work, artifacts stay in a
bounded inspection context; otherwise the skill limits inspection to the
smallest necessary image or crop.

Requires `peekaboo` and `jq` on macOS.

Commands:

- `/peekaboo-macos-validator:peekaboo-doctor`
- `/peekaboo-macos-validator:validate-macos-app <bundle-id>`
