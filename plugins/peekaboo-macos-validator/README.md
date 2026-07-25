# Peekaboo macOS Validator

Runtime automation and visual verification for native SwiftUI and AppKit apps.

Peekaboo is a UI-only fallback. Project commands, tests, app APIs, deep links,
filesystem/process tooling, and macOS CLIs take precedence when they can produce
the requested result without driving the interface.

When UI evidence is required, the plugin prefers accessibility state and
semantic actions. Screenshots are captured only when pixels answer a real
question. When the host supports isolated visual work, artifacts stay in a
bounded inspection context; otherwise the skill limits inspection to the
smallest necessary image or crop.

Requires `peekaboo` and `jq` on macOS.

Commands:

- `/peekaboo-macos-validator:peekaboo-doctor`
- `/peekaboo-macos-validator:validate-macos-app <bundle-id>`
