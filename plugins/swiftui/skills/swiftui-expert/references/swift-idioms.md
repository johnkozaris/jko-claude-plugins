# Modern Swift Idioms

Swift one-liners and stylistic preferences that the SwiftUI community has converged on. These are easy to overlook but compound across a codebase.

## Type System

- **`Double` over `CGFloat`** — Swift bridges them transparently. The two exceptions: `inout` parameters and optionals (`CGFloat?`), where the bridge does not apply.
- **Static member lookup over struct instances** in modifier arguments:

  ```swift
  // Prefer
  .clipShape(.circle)
  .buttonStyle(.borderedProminent)
  .toolbar { ToolbarItem(placement: .topBarTrailing) { ... } }

  // Avoid
  .clipShape(Circle())
  .buttonStyle(BorderedProminentButtonStyle())
  ```

- **Make types `Comparable`** if the same sort closure appears in 2+ places: `books.sorted()` beats `books.sorted { $0.author < $1.author }` everywhere.

## Optionals & Errors

- **`if let value {` shorthand** over `if let value = value {`.
- **Avoid force unwraps (`!`) and `try!`** unless failure is unrecoverable. Even then, prefer `fatalError("clear message")` over a bare `!`.
- **Never silently swallow user-visible errors.** `print(error.localizedDescription)` from a button action is a bug — surface it via alert, toast, or error state.

## Expressions over Statements

- **Omit `return`** in single-expression functions and computed properties.
- **`if`/`switch` as expressions** when assigning or returning:

  ```swift
  // Bad
  var tileColor: Color {
      if isCorrect {
          return .green
      } else {
          return .red
      }
  }

  // Good
  var tileColor: Color {
      if isCorrect { .green } else { .red }
  }
  ```

## Strings, Numbers, Dates

- **`localizedStandardContains()`** for user-input text search. Not `contains()`, not `localizedCaseInsensitiveContains()` — `localizedStandardContains` handles diacritics, case, and width-insensitivity correctly across locales.
- **Never use C-style format strings** (`String(format: "%.2f", value)`). Use `FormatStyle`:
  ```swift
  Text(value, format: .number.precision(.fractionLength(2)))
  Text(price, format: .currency(code: "USD"))
  Text(date, format: .dateTime.day().month().year())
  ```
- **`Date.now`** over `Date()`.
- **Year format**: when a manual format string is unavoidable, use `"y"` not `"yyyy"` — it's correct in all calendars/locales.
- **Date parsing**: `Date(string, strategy: .iso8601)` over `DateFormatter`.
- **`PersonNameComponents`** over manual `"\(first) \(last)"` for people's names — handles localization and ordering.
- **`count(where:)`** over `filter { ... }.count`.

## Collections

- `enumerated()` works directly in `ForEach` — no need to wrap in `Array()`:
  ```swift
  ForEach(items.enumerated(), id: \.element.id) { index, item in ... }
  ```
- Prefer `Identifiable` conformance over `id: \.someProperty` in `ForEach`.

## URLs and Files

- **`URL.documentsDirectory`** / `.cachesDirectory` / `.applicationSupportDirectory` over `FileManager` lookups.
- **`url.appending(path: "subdir")`** over string concatenation or `appendingPathComponent(_:)`.

## Foundation Replacements

- **`replacing("a", with: "b")`** over `replacingOccurrences(of:with:)`.
- **`Subprocess`** package for running external processes (Swift 6.2+).

## Imports

- When `import SwiftUI` is present, **do not also `import UIKit` or `import AppKit`** — they come along automatically on the right platform. `UIImage`, `NSImage`, `UIColor`, etc. are already visible.
- `import Combine` IS still required if you use `ObservableObject`/`@Published`/Combine publishers — that's no longer transitive through SwiftUI.

## Concurrency Quick Wins

- `Task.sleep(for: .seconds(1))` — never `Task.sleep(nanoseconds:)`.
- Prefer `Task { ... }` over `Task.detached { ... }`. `Task.detached` loses actor context and is almost always wrong.
- If an API offers both `async` and closure variants, always use `async`.

## Button Actions

When the action is a method, pass it as a reference instead of wrapping in a trailing closure:

```swift
// Prefer
Button("Save", systemImage: "tray", action: save)

// Avoid (extra closure for no reason)
Button("Save", systemImage: "tray") { save() }
```
