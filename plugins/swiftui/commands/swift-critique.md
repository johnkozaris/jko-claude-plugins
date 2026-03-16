---
description: Critique SwiftUI code for patterns, design, clean code, accessibility, and performance
allowed-tools:
  - Read
  - Glob
  - Grep
  - Skill
argument-hint: "[file-or-directory]"
---

# SwiftUI Critique

Load the `swiftui-expert` skill for reference knowledge, then perform a comprehensive critique of SwiftUI code.

## What to Critique

If `$ARGUMENTS` specifies a file or directory, critique that. If no arguments, find Swift files in the current project:

1. Use Glob to find `**/*.swift` files
2. Use Read to examine each SwiftUI file (files importing SwiftUI or containing `View` conformances)
3. Skip non-SwiftUI files (pure model/utility code without UI)

## Critique Process

Review each file against these categories in order:

### 1. Deprecated API
- Flag any deprecated API usage (foregroundColor, NavigationView, ObservableObject, etc.)
- Show the modern replacement with before/after code

### 2. State & Data Flow
- Verify `@State` is `private`
- Check `@Observable` classes are `@MainActor`
- Flag `Binding(get:set:)` in body
- Flag `@AppStorage` inside `@Observable`
- Check for broad observation dependencies

### 3. View Composition & Clean Code
- Flag computed properties returning `some View` — must be separate View structs
- Flag excessively long `body` properties (40+ lines)
- Check for DRY violations (repeated styling/layout)
- Check Single Responsibility (views doing too many things)
- Flag multiple types in one file
- Flag business logic in body/task/onAppear

### 4. Design Craft & AI Slop
- Flag hardcoded font sizes — must use Dynamic Type
- Flag hardcoded colors that should be semantic
- Flag missing design token usage (scattered magic numbers)
- Check for 44x44 minimum tap targets
- Flag UIKit colors in SwiftUI code
- Flag AI giveaways: gratuitous gradients/materials as backgrounds, heavy shadows on every element, cards wrapping everything, bouncy spring animations on non-interactive elements, web-style CTA buttons, custom nav/tab bars when system ones work

### 5. Animation & Motion
- Flag `.animation()` without `value:` parameter
- Flag manual `animatableData` when `@Animatable` macro would work
- Check for Reduce Motion respect

### 6. Accessibility
- Flag icon-only buttons without text labels
- Flag `onTapGesture` where `Button` should be used
- Flag decorative images without `accessibilityHidden`
- Check for color-only information

### 7. Performance
- Flag `AnyView` usage
- Flag expensive work in body (sorting, filtering, formatter creation)
- Flag eager stacks with large data sets
- Flag stored escaping `@ViewBuilder` closures

### 8. Concurrency
- Flag GCD usage (DispatchQueue)
- Flag `Task.sleep(nanoseconds:)`
- Check for unprotected shared mutable state
- Flag `onAppear` with async work (should be `task()`)

## Output Format

Organize findings by file. For each issue:
1. State the file and relevant line(s)
2. Name the rule being violated
3. Show a brief before/after code fix

Skip files with no issues. End with a prioritized summary of the most impactful changes.

### Example output:

**ContentView.swift**

**Line 12: Use `foregroundStyle()` instead of `foregroundColor()`**
```swift
// Before
Text("Hello").foregroundColor(.red)

// After
Text("Hello").foregroundStyle(.red)
```

**Line 24: Icon-only button — bad for VoiceOver**
```swift
// Before
Button(action: addUser) { Image(systemName: "plus") }

// After
Button("Add User", systemImage: "plus", action: addUser)
```

### Summary
1. **Accessibility (high):** Icon-only button on line 24 is invisible to VoiceOver
2. **Deprecated API (medium):** `foregroundColor()` on line 12 should be `foregroundStyle()`
