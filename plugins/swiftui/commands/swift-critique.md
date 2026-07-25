---
description: Review SwiftUI code using the project's actual targets and the swiftui-expert skill's opinionated, on-demand guidance
argument-hint: "<target> [focus]"
---

# SwiftUI Critique

Review `$ARGUMENTS` as a senior Apple-platform engineer.

Invoke `swiftui-expert` and follow its project-inspection and on-demand reference
guidance. Do not walk every category or manufacture findings to fill a rubric.

Prioritize state and lifecycle correctness, data safety, accessibility,
platform behavior, and user-visible regressions. Distinguish a demonstrated bug
from a migration preference. Respect a coherent existing design unless changing
it solves a concrete problem.

For each material finding, cite the file and line, explain the consequence, give
the smallest coherent fix, and state how it can be verified. Include important
decisions you deliberately left alone when that context prevents unnecessary
rewrites.

When the target is runnable and an existing build or test can validate a
material finding, run the smallest one and state what it did and did not cover.
Use an available runtime validator when needed; otherwise mark runtime behavior
unverified and provide a focused manual check.
