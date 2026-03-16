# Safe Dead Code Removal Strategies

How to remove dead code without breaking things. Measure twice, cut once.

## The Safety Spectrum

Not all dead code carries the same removal risk:

| Risk Level | Category | Strategy |
|---|---|---|
| Zero risk | Unused local variables, unreachable code after return | Delete immediately |
| Low risk | Unused private functions/methods, unused imports | Delete, run tests |
| Medium risk | Unused exported functions in applications | Search project-wide, delete, run tests |
| High risk | Unused public API in libraries | Check downstream consumers before removing |
| Highest risk | Code that "looks" unused but may be invoked dynamically | Investigate thoroughly, see false-positives.md |

## Step-by-Step Safe Removal Process

### Step 1: Verify it's dead
- Run project-wide search: `rg -w 'symbol_name'`
- Check for dynamic references: string literals matching the symbol name
- Check framework conventions: is this a lifecycle hook, convention-based handler, or serialization target?
- Check git blame: is this recent work-in-progress?

### Step 2: Understand why it exists
- Run `git log --follow -p -- file.ext` to see the history
- Run `git log -p --all -S 'symbol_name'` to find when it was added and why
- Read the commit message -- was this part of a feature that was reverted?
- Check for related issues/PRs referenced in the commit

### Step 3: Remove incrementally
- Remove one category at a time (imports first, then variables, then functions)
- Make a dedicated commit for dead code removal (not mixed with feature work)
- Keep the commit message clear: "Remove unused function X" not "cleanup"

### Step 4: Verify nothing broke
- Run the full test suite
- Run the build/compile step
- For web projects: check that the app starts and key routes work
- For libraries: ensure all public API examples still compile

### Step 5: Deploy with monitoring
- For high-risk removals, deploy behind a feature flag or canary
- Monitor error rates after deployment
- Keep the removal commit isolated so it's easy to revert

## The Incremental Approach

Don't try to remove all dead code at once. Prioritize by category:

### Phase 1: Zero Risk (do first)
- Unused local variables
- Unreachable code after return/break/throw
- Unused imports (with side-effect awareness)
- `console.log` / `print()` / `dbg!()` debug artifacts

### Phase 2: Low Risk
- Unused private functions and methods
- Commented-out code blocks
- Lint suppressions (`#[allow(dead_code)]`, `eslint-disable`)
- Skipped tests (either fix or delete)

### Phase 3: Medium Risk
- Unused exported functions in application code
- Unused classes and types
- Orphaned files (test files, config files)
- Unused dependencies (npm, pip, cargo)

### Phase 4: High Risk (careful)
- Unused public API in libraries (SemVer implications)
- Code that might be used via reflection/serialization
- Code in shared packages used by other teams

## Special Situations

### Library Public API
Removing public API is a **breaking change** that requires a major version bump (SemVer). Options:
1. **Deprecate first:** Add `@deprecated` / `#[deprecated]` annotations, release a minor version
2. **Wait a release cycle:** Give consumers time to migrate
3. **Remove in next major:** Bundle removals into a major version release

### Monorepo Cross-Package Dependencies
Before removing code in a shared package:
1. Search ALL packages in the monorepo, not just the current one
2. Check if any CI pipeline or build script references it
3. Check if any configuration file references it

### Dead Code With Tests
If dead code has tests, the tests are also dead. Remove both together. Dead tests:
- Waste CI time
- Inflate code coverage numbers falsely
- Create maintenance burden

### Feature Flags
Code behind permanently-off feature flags is dead code. But verify:
1. Is the flag truly permanent? Check flag management system.
2. Is there a planned rollout? Check product roadmap.
3. Has the flag been off for >6 months with no plans? It's dead.

### Database Migrations
Migration files should generally NOT be deleted even if the tables they create have been removed. Migrations form a chain -- deleting one breaks the chain for fresh installs.

## Refactoring Techniques for Removal

### Remove Dead Code (Fowler)
Simply delete the unreferenced code. The simplest refactoring.

### Collapse Hierarchy
When a superclass and subclass are not different enough to justify separate classes, merge them.

### Inline Class
When a class is barely doing anything (perhaps after other refactorings removed most functionality), merge it into its only caller.

### Inline Method
When a function body is as clear as its name, replace all calls with the body and delete the function.

### Remove Parameter
When a parameter is not used by the function body, remove it from the signature and all call sites.

## Meta's SCARF Approach (Scale)

Meta's Systematic Code and Asset Removal Framework operates at massive scale:
1. **Build dependency graph** from compilers (static analysis)
2. **Augment with runtime data** from production logs (dynamic analysis)
3. **Detect unreachable nodes** including cycles in the dependency graph
4. **Auto-generate removal PRs** on a daily basis
5. **Human review** catches false positives, which improve the analysis

Key insight: SCARF combines static AND dynamic analysis. Static analysis finds code with no references. Dynamic analysis (production logs) finds code with references but zero runtime execution. The combination is more powerful than either alone.

Result: 100M+ lines deleted, 370K+ change requests, over 5 years.

## Metrics to Track

- **Dead code ratio:** Lines of dead code / total lines (aim for <5%)
- **Removal velocity:** Lines of dead code removed per sprint
- **Zombie age:** Average age of dead code in the repo (older = more confidence it's dead)
- **Lint suppression count:** Number of dead-code-related lint suppressions (should trend to zero)
