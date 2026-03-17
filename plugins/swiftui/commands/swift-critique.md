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

Load the `swiftui-expert` skill for reference knowledge, then perform a grounded critique of SwiftUI code.

## Gather Context First

Before flagging anything, understand the project:

1. Check deployment target from `Package.swift`, `*.xcodeproj/project.pbxproj`, or `*.xcconfig` files
2. Check if the project uses `@Observable` or `ObservableObject` (don't flag `ObservableObject` if the whole project consistently uses it and targets < iOS 17)
3. Check if the project has a design system / theme (shared colors, fonts, spacing constants)
4. Note any architectural patterns already in use (MVVM, MV, coordinator, etc.)

## What to Critique

If `$ARGUMENTS` specifies a file or directory, critique that. If no arguments, find Swift files in the current project:

1. Use Glob to find `**/*.swift` files
2. Use Read to examine each SwiftUI file (files importing SwiftUI or containing `View` conformances)
3. Skip non-SwiftUI files (pure model/utility code without UI)

## Critique Rules

- **Read the actual code before flagging.** Do not flag something you haven't verified by reading the file.
- **Respect project conventions.** If the project consistently uses a pattern (e.g., `ObservableObject`, computed view properties), don't flag every instance — note it once as a project-wide suggestion.
- **Verify before flagging.** If you think something is unused or misused, check the call sites. An `@Environment` property added in the same session may be used in code you haven't read yet.
- **No generic advice.** Every finding must reference a specific file and line number with actual code from the project.

## Critique Process

Review each file against these categories in order:

### 1. Deprecated API
- Flag any deprecated API usage (foregroundColor, NavigationView, ObservableObject, etc.)
- Show the modern replacement with before/after code

### 2. State & Data Flow
- Verify `@State` is `private` (skip if the project has a pattern of non-private `@State` for parent injection)
- Check `@Observable` classes are `@MainActor` (only if project targets iOS 17+)
- Flag `Binding(get:set:)` in body
- Flag `@AppStorage` inside `@Observable`
- Check for broad observation dependencies (verify by reading the model, not guessing)

### 3. View Composition & Clean Code
- Flag computed properties returning `some View` only if they contain state-dependent logic or are reused — short helper properties are fine
- Flag `body` over 60 lines (not 40 — short views with modifier chains are normal)
- Check for DRY violations only if you find 3+ instances of repeated styling/layout
- Flag multiple public types in one file (private helper types in the same file are fine)
- Flag business logic (network calls, data transforms) directly in body/task/onAppear

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
- Flag GCD usage (`DispatchQueue.main.async`, etc.) — unless wrapping UIKit interop that requires it
- Flag `Task.sleep(nanoseconds:)` — suggest `Task.sleep(for:)`
- Flag `onAppear` containing `Task { }` — suggest `.task { }` modifier instead

## Output Format

Organize findings by file. For each issue:
1. State the file and relevant line(s)
2. Name the rule being violated
3. Show a brief before/after code fix

Skip files with no issues. Do not pad the report with minor nits — only report findings that materially improve the code. End with a prioritized summary of the most impactful changes.

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
