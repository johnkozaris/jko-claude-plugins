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
function isEven(n) {
  function checkEvenness(num) {
    return n % 2 === 0;
    return (num & 1) === 0;
  }
}
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

| Tool      | Languages         | Clone Types   | Notes                                   |
| --------- | ----------------- | ------------- | --------------------------------------- |
| jscpd     | Any (token-based) | 1, 2          | Cross-language, configurable thresholds |
| PMD CPD   | Java, JS, others  | 1, 2          | Part of PMD suite (token-based)         |
| Simian    | Any               | 1, 2          | Commercial, very configurable           |
| CloneDR   | Any               | 1, 2, 3       | AST-based, commercial                   |
| SonarQube | Multi-language    | 1, 2, 3       | Enterprise code quality platform        |
| Semgrep   | Multi-language    | Pattern-based | Write custom duplication rules          |

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

---

## Cross-Boundary Duplication ("Split-Brain")

The most expensive form of duplication: same logic implemented twice across an architectural seam (client/server, service/service, database/code, language/language). **Both copies look alive** — drift produces production bugs, not unused code. Static analysis at the file or module level cannot see these because the duplicates live in separate dependency graphs.

The Fowler/Refactoring.Guru name closest to this is **"Alternative Classes with Different Interfaces"**, but the patterns below extend that across processes, services, and runtimes.

### Pattern: Validation Duplicated Client ↔ Server

The same input rules implemented in JavaScript on the frontend and Python/Go/C#/Rust on the backend.

**Example:**

```typescript
// frontend/schemas.ts
export const userSchema = z.object({
  email: z.string().email(),
  age: z.number().int().min(13).max(120),
});
```

```python
# backend/schemas.py
class User(BaseModel):
    email: EmailStr
    age: int = Field(ge=13, le=120)
```

**Drift mode:** Backend updates `min(13)` to `min(18)` for COPPA compliance; frontend doesn't. Users get cryptic 422 errors. Or frontend tightens but backend stays loose — security regression.

**Detection:**

```bash
# Look for same field with constraints in both halves of the repo
rg -n '(min|max|minLength|maxLength|ge|le|gt|lt)\s*[(:=]\s*\d+' \
  -g 'frontend/**' -g 'backend/**' -g 'client/**' -g 'server/**'
```

**Fix:**

- **Single source of truth via codegen:** OpenAPI / JSON Schema / Protobuf / GraphQL → generate both client and server validators.
- **Shared TypeScript schema** (zod, valibot) consumed by both Node backend and frontend.
- **Cross-language schema language** (e.g., Smithy, JSON Schema with `quicktype`).

### Pattern: DTO ↔ Entity ↔ Frontend Type Triplet

The same data shape declared in 3+ places: ORM entity, API DTO, OpenAPI schema, TypeScript interface.

**Example:** `User` exists as

- `models/user.py` (SQLAlchemy entity, 8 fields)
- `schemas/user.py` (Pydantic DTO, 7 fields — minus password)
- `openapi.yaml` `components.schemas.User` (auto-generated, sometimes stale)
- `frontend/types/user.ts` (hand-written, 6 fields — minus internal flags)

**Drift mode:** Add a field to the entity; forget the DTO; field is silently absent from API responses for users on the frontend.

**Detection:**

```bash
# Find the same field name set across model/dto/type directories
for field in id email name created_at; do
  echo "=== $field ==="
  rg -l "\b$field\b" -g 'models/**' -g 'schemas/**' -g 'dto/**' -g 'types/**' -g '*.ts' -g '*.py'
done
```

**Fix:**

- Generate DTOs from entities (Pydantic from SQLAlchemy via `sqlmodel`, EF Core entities → DTOs via Mapster/AutoMapper with explicit profiles).
- Generate frontend types from OpenAPI (`openapi-typescript`, `orval`, `kubb`).
- Use a single schema language (Protobuf with codegen for all three layers).

### Pattern: Two Services Solving the Same Domain Problem

Microservice extraction left an orphaned implementation in the monolith, or two teams independently built the same capability.

**Example:** `OrderService` in monolith and `OrderProcessor` in new microservice both write to the `orders` table.

**Drift mode:** Monolith writes use a different status enum value; reconciliation jobs paper over the difference; eventually a state nobody handles appears.

**Detection:**

- Both services connecting to the same database table or topic.
- Two API endpoints (often different versions or different services) covering the same resource.
- Two background workers subscribed to the same event type.

```bash
# Find services touching the same table
rg -n "\b(orders|order_items)\b" --type sql --type py --type cs --type ts -l \
  | sort -u
```

**Fix:** Decide canonical owner; deprecate the other with a timer; route all traffic through the canonical service before deletion. **Removing a service is a high-risk operation — use the SCARF approach (`safe-removal.md`).**

### Pattern: Read Path / Write Path Drift (CQRS Done Wrong)

Command model writes a field; query model loses it during projection.

**Example:** Commands write `Order.customer_loyalty_tier`; the read model projection only copies `Order.customer_id`. Querying the read model gives `null` for tier even though it's in the source.

**Detection:** Compare command-side fields vs read-model fields for the same entity. Look for entity DTOs in `commands/` vs `queries/` or `write/` vs `read/` directories.

**Fix:** Either drop the field from the write model (it's dead) or add it to the read projection (it's missing). Don't leave the asymmetry — it's a future bug.

### Pattern: Two Configuration Sources for the Same Setting

Same setting populated from `config.yaml` and an environment variable, with one silently winning.

**Example:**

```python
DATABASE_URL = config.get("database.url") or os.getenv("DATABASE_URL")
```

- In dev, `config.yaml` wins (env not set).
- In prod, env wins (k8s injects it).
- Operator changes `config.yaml` in prod expecting it to take effect — nothing happens. Hours of debugging.

**Detection:**

```bash
# Find same key referenced from both env and config loaders
rg -o "['\"]([A-Z_][A-Z0-9_]+)['\"]" -t py -t ts | grep -E '(getenv|environ|env\.)' | sort -u
rg -o "['\"]([a-z_.]+)['\"]" -t py -t ts | grep -E '(config\.get|GetSection|load_config)' | sort -u
# Cross-reference manually
```

**Fix:** Pick one source per setting. Document precedence loudly if both are intentional (e.g., env > file is a common pattern, but document it in README and in the loader's docstring).

### Pattern: Old API + New API Both Live

`/v1/users` and `/v2/users` both serving traffic with no deprecation timer.

**Detection:**

- Walk route table; group by resource; if multiple versions exist for the same resource, check access logs for v1 traffic. If v1 has near-zero traffic, it's dead.
- Look for `@deprecated` / `[Obsolete]` / `#[deprecated]` annotations without a removal date.

**Fix:** Set a deprecation date. Notify consumers. Use access logs (Meta SCARF style) to confirm zero callers, then remove. See `safe-removal.md` "Library Public API" section.

### Pattern: Frontend Reimplements Backend Computation

Backend computes `total = subtotal + tax - discount`; frontend reimplements the same formula for the order preview.

**Drift mode:** Backend adds a "loyalty discount" line item; frontend preview disagrees with the actual charge.

**Detection:** Same constants (tax rates, fees, formulas) appearing in both stacks. Look for hard-coded numbers in `frontend/` and `backend/`.

**Fix:**

- Backend computes a "preview" endpoint the frontend calls.
- Or: extract pure computation to a shared library compiled to both runtimes (e.g., Rust → WASM for browser + native for backend; or shared TypeScript run on both Node and browser).

### Pattern: Duplicate Enums Across Boundaries

Same enum hand-maintained in TypeScript, C#, Python, Swift.

**Example:**

```typescript
enum Status {
  PENDING,
  ACTIVE,
  CANCELLED,
}
```

```python
class Status(Enum):
    PENDING = "PENDING"; ACTIVE = "ACTIVE"; CANCELLED = "CANCELLED"
```

```swift
enum Status: String { case pending, active, cancelled }
```

**Detection:**

```bash
# Find enum-like declarations of the same values across languages
rg -o '(enum|class)\s+(\w+)' -t ts -t py -t cs -t swift --no-filename \
  | sort | uniq -c | sort -rn | awk '$1 > 1'
```

**Fix:** Generate from a single source (Protobuf enums, OpenAPI schema enums, Smithy). When the canonical change happens (add a `RETURNED` status), all stacks regenerate together.

### Pattern: Mobile App ↔ Web App Reimplementing Each Other

iOS, Android, and web all implement the same business logic three times. Bug fixed on web takes weeks to land on mobile.

**Detection:** Cross-platform repos with `ios/`, `android/`, `web/` directories containing the same domain rules. Hard to detect automatically; flag during architecture review.

**Fix:** Move business logic to a backend "thin client" model (mobile/web are thin), or to a shared core (Kotlin Multiplatform, Rust core via FFI, TypeScript shared package).

### Pattern: Schema Defined in Migrations and Models

The DB schema is "the migrations" but also "the ORM model definitions" — and they can disagree (especially after manual hot-fixes in production).

**Detection:** Run `manage.py makemigrations --dry-run` (Django) or `dotnet ef migrations add --no-build Test` (EF Core) — non-empty output means model and DB schema have drifted.

**Fix:** Treat one as canonical. For most projects: ORM model is canonical, migrations are derived. Reject manual schema edits in production.

---

## Split-Brain Detection Heuristics (Cross-Stack Quick Scan)

```bash
# 1. Same constant in multiple languages
rg -o '\b\d+\.\d+\b|\b[A-Z]{2,}_[A-Z_]+\s*=' -t ts -t py -t cs -t rs -t swift --no-filename \
  | sort | uniq -c | sort -rn | awk '$1 >= 2 {print $1, $2}' | head -30

# 2. Same field name across model/dto/schema/type directories
for d in models schemas dto types entities; do
  fd -t f . "$d" 2>/dev/null
done | xargs rg -o '^\s*(\w+):' --no-filename -r '$1' 2>/dev/null \
  | sort | uniq -c | sort -rn | awk '$1 >= 3' | head -30

# 3. Two route handlers covering the same resource path
rg -o '(get|post|put|delete|patch|map_get|map_post|MapGet|MapPost|app\.route|@route|@app\.(get|post))\([\'"]([^\'"]+)' \
  --no-filename -r '$2' \
  | sort | uniq -c | sort -rn | awk '$1 >= 2'

# 4. Hardcoded URLs / endpoints duplicated
rg -o 'https?://[a-zA-Z0-9./?=&_-]+' --no-filename | sort | uniq -c | sort -rn | head -20
```

---

## When to Tolerate Cross-Boundary Duplication

Not every duplication is a split-brain bug. Acceptable cases:

- **Performance:** Frontend computes a preview locally to avoid a server round-trip; backend recomputes for authority. Document this and add a "preview ↔ authority must match" test.
- **Defense in depth:** Validation runs on both sides because the frontend can be bypassed. Document that the backend is canonical.
- **Generated code:** Two implementations exist but both come from a single source (codegen). Verify the generator is in CI.

The danger is **undocumented** duplication that drifts silently. Every cross-boundary duplicate should have either codegen or a comment naming the canonical source and the reason for duplication.
