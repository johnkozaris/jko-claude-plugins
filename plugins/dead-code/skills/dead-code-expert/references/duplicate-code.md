# Duplicate Code & Dual Implementations

Detecting and eliminating code duplication, parallel implementations, and speculative generality.

## Clone Types

Academic research classifies code duplication into four types:

### Type 1: Exact Clones
Identical code fragments except for whitespace, layout, and comments.
```
// Fragment A                    // Fragment B
function add(a, b) {            function add(a, b) {
  return a + b;                   return a + b;
}                                }
```
**Detection:** Simple text comparison after normalizing whitespace.

### Type 2: Renamed Clones
Identical structure but with different variable names, types, or literals.
```python
# Fragment A                     # Fragment B
def calc_tax(price, rate):       def compute_fee(amount, percent):
    return price * rate              return amount * percent
```
**Detection:** Token-based comparison after normalizing identifiers.

### Type 3: Near-Miss Clones
Similar fragments with added, removed, or modified statements.
```rust
// Fragment A                          // Fragment B
fn process_order(order: &Order) {      fn process_refund(refund: &Refund) {
    validate(order);                       validate(refund);
    let total = calculate(order);          let total = calculate(refund);
    log_event("order", total);             log_event("refund", total);
    save(order);                           notify_customer(refund);  // different
}                                          save(refund);
                                       }
```
**Detection:** AST-based comparison with gap tolerance.

### Type 4: Semantic Clones (Dual Implementations)
Different code that does the same thing. This is the "dual brain" problem.
```javascript
// Developer A wrote this          // Developer B wrote this
function isEven(n) {               function checkEvenness(num) {
  return n % 2 === 0;                return (num & 1) === 0;
}                                   }
```
**Detection:** Requires understanding intent. LLMs excel here where tools fail.

## Dual Implementation Patterns

### Pattern: Two Functions, Same Purpose
Two functions that solve the same problem in different ways, often written by different developers.

**How to spot:**
- Similar function names with different wording (`getUser`/`fetchUser`, `parseData`/`processData`)
- Functions with the same parameter types and return type
- Functions in different modules that operate on the same domain concept

**How to fix:** Choose the canonical implementation, redirect all callers, delete the other.

### Pattern: Parallel Class Hierarchies
Two separate class/type hierarchies that mirror each other.

**How to spot:**
- Classes with matching names in different packages (`models.User` and `dto.User` with same fields)
- Converter functions between parallel types that are just field-by-field copies

**How to fix:** Evaluate if both hierarchies are needed. Often one can be eliminated or they can be consolidated.

### Pattern: Redundant Validation
The same validation performed at multiple layers without purpose.

**How to spot:**
- Input validated in the controller, validated again in the service, validated again in the repository
- Null checks repeated at every function boundary
- Type checks that the type system already guarantees

**How to fix:** Validate at system boundaries (API entry points). Trust internal code and type system.

### Pattern: Wrapper Functions Adding No Value
Functions that just call another function with the same arguments.

```python
# Adds nothing
def get_user(user_id):
    return database.get_user(user_id)
```

**How to spot:** Function body is a single call with passthrough arguments. No transformation, no error handling, no additional logic.

**How to fix:** Inline the wrapper. Call the underlying function directly.

### Pattern: Copy-Paste With Slight Modification
Code copied from one place and modified slightly, diverging over time.

**How to spot:**
- Blocks of code with 80%+ structural similarity
- Functions that differ only in one or two lines
- Identical error handling blocks repeated across functions

**How to fix:** Extract shared logic into a function, parameterize the differences.

## Speculative Generality

Code written "just in case" for futures that never arrived.

### Interfaces With Single Implementation
An interface/trait/protocol with exactly one concrete type. Unless it's for testability (dependency injection), it's premature abstraction.

**Detection:**
```bash
# Find interfaces/traits, then count implementations
rg 'interface (\w+)' --type ts -o -r '$1' | sort | while read iface; do
  count=$(rg "implements $iface" --type ts -c | awk -F: '{s+=$2} END {print s+0}')
  [ "$count" -le 1 ] && echo "Single-impl interface: $iface ($count implementations)"
done
```

### Unused Function Parameters
Parameters accepted but never used in the function body. Kept "for future use" or left from refactoring.

**Detection:**
- Most linters flag this: ESLint no-unused-vars with args option, rustc unused_variables, pylint unused-argument
- Parameters prefixed with `_` are intentionally unused (convention in Rust, Python)

### Configuration Nobody Uses
Config options, feature flags, or environment variables that are always set to the same value.

**Detection:** Search for config reads and check if the value ever varies across environments.

### Abstract Factory / Strategy / Visitor With One Variant
Design patterns applied prematurely when only one concrete variant exists.

**How to spot:**
- Factory that creates only one type
- Strategy interface with one implementation
- Visitor with one visit method

**How to fix:** Inline the pattern. Use direct construction/calls. Re-introduce the pattern only when a second variant is needed.

## Detection Tools

| Tool | Languages | Clone Types | Notes |
|---|---|---|---|
| jscpd | Any (token-based) | 1, 2 | Cross-language, configurable thresholds |
| PMD CPD | Java, JS, others | 1, 2 | Part of PMD suite (token-based) |
| Simian | Any | 1, 2 | Commercial, very configurable |
| CloneDR | Any | 1, 2, 3 | AST-based, commercial |
| SonarQube | Multi-language | 1, 2, 3 | Enterprise code quality platform |
| Semgrep | Multi-language | Pattern-based | Write custom duplication rules |

## The DRY Escalation Ladder

When eliminating duplication, escalate only as needed:

1. **Extract function** -- simplest, extract repeated logic
2. **Parameterize** -- make the differing parts parameters
3. **Generics/templates** -- eliminate per-type duplication (zero-cost in Rust/C++)
4. **Trait default methods** -- shared behavior inherited by types
5. **Macros/metaprogramming** -- last resort when language abstractions are insufficient

Three similar lines of code are better than a premature abstraction. Apply the Rule of Three: don't abstract until you see the pattern three times.

## LLM Advantage for Duplicate Detection

Traditional tools excel at Type 1-3 clones (syntactic similarity). LLMs excel at Type 4 (semantic clones) because they understand intent. When reviewing code, actively look for:
- Functions with different names but same purpose
- Different algorithms solving the same problem
- Reimplementations of standard library functionality
- Hand-rolled logic that a well-known library already provides
