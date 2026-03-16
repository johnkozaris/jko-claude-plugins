# SwiftUI Expert Plugin

Expert SwiftUI guidance — modern patterns, design craft, clean code principles, and code critique. Targets the latest iOS and Swift versions.

## Components

### Skill: `swiftui-expert`
Auto-activates when writing, reviewing, or debugging SwiftUI code. Provides guidance on:
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

### Command: `/swift-critique`
Reads SwiftUI code files and critiques them against all rules — deprecated APIs, design craft, clean code, accessibility, performance, concurrency, and AI aesthetics. Shows before/after fixes.

```
/swift-critique                    # Critique all .swift files in project
/swift-critique Sources/Views/     # Critique a specific directory
/swift-critique ContentView.swift  # Critique a single file
```

## Installation

Copy or symlink to your Claude Code plugins directory, or use within the myClaudeSkills marketplace.

## Targets

- Latest iOS / Swift / Xcode (detects from project)

## License

MIT
