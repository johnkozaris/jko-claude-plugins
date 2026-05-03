# SwiftUI Expert Plugin

Expert SwiftUI guidance — modern patterns, design craft, clean code principles, and code critique. Targets the latest iOS and Swift versions.

## What It Does

A production-focused SwiftUI skill that critiques code for modern API usage, state and data flow, clean composition, accessibility, performance, animation, concurrency, navigation, and visual craft. It pushes toward current Apple platform patterns without drifting into stale SwiftUI guidance or generic UIKit advice.

## Installation

```bash
# From the marketplace
claude plugin marketplace add /path/to/myClaudeSkills
claude plugin install swiftui@jko-claude-plugins

# Or load for one session
claude --plugin-dir /path/to/myClaudeSkills/plugins/swiftui
```

## Commands

| Command | Purpose |
|---|---|
| `/swift-critique` | Review SwiftUI code for modern patterns, design quality, accessibility, performance, concurrency, and visual polish |

## Skill

The `swiftui-expert` skill activates automatically when writing, reviewing, or debugging SwiftUI code. It provides:

- Modern API usage (deprecated API replacement)
- State management (`@Observable`, property wrappers, SwiftData)
- View composition and clean code (DRY, SRP, Open/Closed)
- Design craft (typography, color, spacing, visual hierarchy, avoiding AI slop)
- Animation and motion (springs, transitions, Liquid Glass morphing)
- Accessibility (VoiceOver, Dynamic Type, reduce motion)
- Performance (code smells, remediation, debugging)
- Concurrency (async/await, actors, Sendable, Swift 6)
- Navigation (NavigationStack, sheets, tabs, deep links)
- Liquid Glass (iOS 26+ adoption, fallback patterns)

## Hook

No active runtime hooks. `hooks/hooks.json` is reserved for future hook-based checks.

## References

10 reference files organized by domain:

accessibility, animation, concurrency, design-craft, liquid-glass, modern-api, navigation, performance, state-data, view-composition

## Targets

- Latest iOS / Swift / Xcode (detects from project)

## License

MIT
