# AI Slop in .NET Backends

AI-generated backend code often compiles, sometimes even passes tests, and still damages the design. Review for structural fit before style.

## Tells

| ID | Tell | Why It Matters |
|---|---|---|
| AS-01 | interface per class with no real seam | abstraction tax and indirection |
| AS-02 | `BaseService` / `BaseRepository` explosion | cargo-cult architecture |
| AS-03 | giant `Program.cs` with inline everything | no feature structure, unreadable host |
| AS-04 | pass-through services | fake layering with zero value |
| AS-05 | endpoints or hubs touch infrastructure directly | layer violation |
| AS-06 | same type used for entity, request, response, and persistence | boundary collapse |
| AS-07 | generic helper / manager / provider names everywhere | no domain language |
| AS-08 | tests mirror implementation details only | coverage theater |
| AS-09 | heavy comments restating obvious code | generated noise instead of design |
| AS-10 | random factories and strategy patterns with one implementation | ceremony before need |
| AS-11 | static helper bags full of hidden dependencies | anti-DI design |
| AS-12 | eventing/AppHost/microservices added with no clear problem | modernization theater |

## Scorecard

### Clean

0-2 tells. The code looks designed.

### Moderate

3-5 tells. The code likely needs architectural cleanup before more features land.

### Heavy

6+ tells. Treat the code as assembled rather than designed and review every boundary carefully.

## Recovery Strategy

1. Collapse meaningless wrappers.
2. Move business logic into the right boundary.
3. Remove fake abstractions.
4. Introduce explicit contracts.
5. Rebuild one coherent feature slice end-to-end.
6. Add tests that prove behavior, not scaffolding.

## Review Questions

- Which abstractions are protecting a real seam, and which are just there?
- Can I follow one request from endpoint to persistence without jumping through decorative layers?
- Does the code use domain language, or generic AI-shaped nouns like manager/provider/helper?
