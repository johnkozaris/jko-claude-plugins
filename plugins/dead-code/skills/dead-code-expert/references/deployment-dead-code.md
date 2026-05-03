# Deployment & Asset Dead Code

Dead code that lives outside source files. In modern apps this is often the largest category of cruft and the hardest to detect because no compiler complains. Covers assets, configuration, infrastructure, schemas, routes, docs, and CI.

Use this reference when scanning real-world projects, not just source folders.

---

## 1. Environment Variables

**What:** Variables set in `.env`, `.env.example`, CI secrets, Helm values, Docker Compose, Kubernetes ConfigMap/Secret — but never read by any process.

**Why it matters:** Stale env vars mislead operators, leak credentials, complicate deploys, and indicate features that were rolled back without cleanup.

**Detection:**

```bash
# Collect declared env var names from all sources
{
  rg -o '^[A-Z_][A-Z0-9_]+=' .env* 2>/dev/null | sed 's/=$//'
  rg -o '^\s*-?\s*name:\s*([A-Z_][A-Z0-9_]+)' -r '$1' k8s/ helm/ 2>/dev/null
  rg -o '\$\{?([A-Z_][A-Z0-9_]+)\}?' docker-compose*.yml 2>/dev/null
} | sort -u > /tmp/declared_envs.txt

# Collect read sites in source
{
  rg -o "os\.environ\[['\"]([A-Z_][A-Z0-9_]+)['\"]\]" -r '$1' -t py
  rg -o "os\.getenv\(['\"]([A-Z_][A-Z0-9_]+)['\"]" -r '$1' -t py
  rg -o "process\.env\.([A-Z_][A-Z0-9_]+)" -r '$1' -t js -t ts
  rg -o 'std::env::var\("([A-Z_][A-Z0-9_]+)"\)' -r '$1' -t rust
  rg -o 'Environment\.GetEnvironmentVariable\("([A-Z_][A-Z0-9_]+)"' -r '$1' -t cs
  rg -o 'ProcessInfo\.processInfo\.environment\["([A-Z_][A-Z0-9_]+)"' -r '$1' -t swift
} | sort -u > /tmp/read_envs.txt

# Diff: declared but never read
comm -23 /tmp/declared_envs.txt /tmp/read_envs.txt
```

**Caveats:** Some env vars are read by the runtime (e.g., `NODE_ENV`, `PYTHONPATH`, `DATABASE_URL` consumed by ORM drivers) — maintain an allowlist.

---

## 2. Configuration Files

**What:** Keys in `appsettings.json`, `config.yaml`, `pyproject.toml [tool.*]`, `package.json` script entries, etc. that nothing reads.

**Detection per stack:**

- **.NET `appsettings.json`**: Walk the JSON tree, grep each leaf key as a string literal: `rg "GetSection\(\"$KEY\"\)|GetValue<.*>\(\"$KEY\"\)" -t cs`.
- **Python `pyproject.toml`**: Tool sections (`[tool.foo]`) for tools no longer in the dependency list.
- **package.json `scripts`**: Scripts never invoked by CI, README, or other scripts: `rg "npm run $SCRIPT|pnpm $SCRIPT|yarn $SCRIPT"`.
- **YAML/TOML configs**: Walk all keys, grep string literals across source.

**Special case — feature flags:**

- Permanently-on flags: `if (FLAG_X) { ... }` where `FLAG_X` has been `true` in every environment for >6 months.
- Permanently-off flags: same, but always `false`. The `then` branch is dead.
- Detection: query the flag management system (LaunchDarkly, Unleash, Statsig) for flags with no recent variation; cross-check against codebase.

---

## 3. Container & Image Dead Code

### Dockerfile

**Patterns:**

- Multi-stage stages whose output is never `COPY --from=<stage>`'d.
- `RUN` commands that install packages no later step uses.
- `COPY` of files into the image that no process reads at runtime.
- `EXPOSE` ports the app doesn't bind to.
- `ENV` declarations no process reads.
- `ARG` build args set in CI but never referenced in the Dockerfile body.

**Detection:**

```bash
# List multi-stage stage names
rg '^FROM .* AS (\w+)' -r '$1' Dockerfile

# Cross-check against COPY --from=
rg 'COPY --from=(\w+)' -r '$1' Dockerfile

# Stages declared but never copied from = dead (unless the final stage)
```

**Tools:** `dive` (per-layer file analysis), `hadolint` (Dockerfile lint, catches some dead patterns), `docker-slim` (auto-trim images).

### Kubernetes / Helm

**Patterns:**

- `Service` pointing at a `selector` no `Pod` matches.
- `ConfigMap` keys never mounted into any container.
- `Secret` keys never injected as env vars or mounted.
- `ServiceAccount` / `Role` / `RoleBinding` / `ClusterRole` granting permissions no workload uses.
- `Ingress` rules to backends that no longer exist.
- `HorizontalPodAutoscaler` for a `Deployment` that's been scaled to 0.
- Helm chart values in `values.yaml` never templated into any manifest.

**Detection:**

```bash
# Helm: values declared but not used
helm template chart/ | grep -oE '\.Values\.[a-zA-Z._]+' | sort -u > /tmp/used_values.txt
yq eval '.. | path' chart/values.yaml | sort -u > /tmp/declared_values.txt
comm -23 /tmp/declared_values.txt /tmp/used_values.txt
```

**Tools:** `kube-score`, `kubeval`, `polaris`, `datree`.

### Terraform / IaC

**Patterns:**

- Resources defined with `count = 0` permanently.
- Modules in `modules/` directory with no `module "..."` callers.
- Variables declared in `variables.tf` never referenced.
- Outputs that nothing consumes (no `terraform_remote_state` reader).
- Provider configurations for unused providers.

**Detection:**

```bash
# Find unused variables
rg -o '^\s*variable\s+"(\w+)"' -r '$1' --no-filename | sort -u > /tmp/declared_vars.txt
rg -o 'var\.(\w+)' -r '$1' --no-filename | sort -u > /tmp/used_vars.txt
comm -23 /tmp/declared_vars.txt /tmp/used_vars.txt
```

**Tools:** `tflint` (built-in unused variable check), `terraform-compliance`, `checkov`.

---

## 4. CI / CD Pipelines

**Patterns:**

- Reusable workflows / composite actions never called.
- Jobs that produce artifacts no downstream job consumes.
- Cache keys for caches nothing restores.
- Build matrix entries (OS, version) that fail and have been allowed-to-fail forever.
- Secrets declared in repo settings but no workflow references.
- Scheduled workflows (`on: schedule:`) for jobs whose business reason is gone.

**Detection:**

```bash
# GitHub Actions: list referenced reusable workflows
rg 'uses:\s+([^\s@]+)' -r '$1' .github/workflows/ | sort -u > /tmp/used_actions.txt

# Composite actions in repo
fd -t f action.yml .github/actions/ | xargs -I{} dirname {} | xargs -I{} basename {} | sort -u > /tmp/declared_actions.txt

# Diff
comm -23 /tmp/declared_actions.txt /tmp/used_actions.txt
```

For unused secrets, query `gh secret list` and grep workflow files for each name.

---

## 5. Database

**Patterns:**

- Tables nothing reads from or writes to.
- Columns nothing references in queries.
- Indexes that no query plan uses (the index is maintained on every write for nothing).
- Stored procedures / functions nobody calls.
- Triggers firing for events nothing observes.
- Materialized views never queried.
- Migration files for entities since deleted (but **don't delete migrations** — they form a chain).

**Detection:**

- **Static (codebase-side):**
  ```bash
  # Get all table names from migrations / schema, then grep ORM/SQL for each
  rg -o 'CREATE TABLE\s+(\w+)' -r '$1' migrations/ db/ | sort -u > /tmp/tables.txt
  while read t; do
    count=$(rg -wc "$t" --type sql --type py --type ts --type cs 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
    [ "$count" -le 1 ] && echo "Possibly dead table: $t (refs: $count)"
  done < /tmp/tables.txt
  ```
- **Dynamic (production):** Query `pg_stat_user_tables` (PostgreSQL) for `seq_scan + idx_scan == 0`. For unused indexes: `pg_stat_user_indexes` `idx_scan == 0`.
- **Tools:** `pg_unused` views, `pt-index-usage` (MySQL).

---

## 6. Public HTTP Routes / API Surface

**Patterns:**

- Server route handlers no client (frontend, mobile, external partner) calls.
- API versions kept alive past their deprecation window.
- OpenAPI / GraphQL schema entries with no resolver consumer or no client query.
- Webhooks declared but no subscriber.

**Detection:**

```bash
# Compare declared server routes to client call sites
# Example: Express
rg -o "app\.(get|post|put|delete|patch)\(['\"]([^'\"]+)['\"]" -r '$2' server/ | sort -u > /tmp/server_routes.txt
rg -o "fetch\(['\"]([^'\"]+)" -r '$1' client/ | sort -u > /tmp/client_calls.txt
comm -23 /tmp/server_routes.txt /tmp/client_calls.txt
```

**Dynamic:** API gateway / load balancer access logs reveal routes with zero traffic over a period. Combine with static analysis (Meta SCARF approach in `safe-removal.md`).

---

## 7. Frontend Routes / Navigation

**Patterns:**

- Routes registered in `react-router` / `next/app` / SwiftUI `NavigationStack` / Android nav-graph that no `<Link>` / `navigate()` / deep link reaches.
- Deep link URI schemes declared but never produced.
- Push notification handlers for notification types nothing sends.
- Menu items / sidebar entries pointing at removed pages.

**Detection:** Walk the route table; for each route, grep for `<Link to=` / `Linking.openURL` / `navigate(` / `router.push(`.

---

## 8. Localization & Assets

**Patterns:**

- Translation keys in `.json`/`.po`/`.strings`/`.xcstrings`/`.xliff` never looked up.
- Image / icon / font / video files in `public/`, `assets/`, `static/` not referenced anywhere.
- Sound effects, haptic patterns, color tokens declared in design system but never imported.
- Email templates / SMS templates for triggers that no longer fire.

**Detection:**

```bash
# Localization keys
jq -r 'keys[]' locales/en.json > /tmp/keys.txt
while read k; do
  count=$(rg -wc "['\"]$k['\"]" -t ts -t tsx -t js -t jsx 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
  [ "$count" -eq 0 ] && echo "Dead key: $k"
done < /tmp/keys.txt

# Asset files
fd -t f -e png -e jpg -e svg -e ico public/ assets/ | while read f; do
  base=$(basename "$f" | sed 's/\.[^.]*$//')
  count=$(rg -wc "$base" -t ts -t tsx -t js -t jsx -t css -t html 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
  [ "$count" -eq 0 ] && echo "Possibly orphan asset: $f"
done
```

---

## 9. Documentation Drift

**Patterns:**

- README sections describing features that were removed.
- API docs (`/docs`, Swagger UI, ReDoc) for endpoints that 404.
- Architecture diagrams referencing services that no longer exist.
- Tutorial / example code that no longer compiles.
- Markdown links to files that have been deleted or renamed.

**Detection:**

```bash
# Broken markdown links
rg -o '\[([^\]]+)\]\(([^)]+)\)' --no-filename -r '$2' -t md | grep -v '^http' | grep -v '^#' | while read link; do
  [ ! -e "$link" ] && echo "Broken doc link: $link"
done

# Or use lychee / markdown-link-check
lychee --no-progress --include-fragments './**/*.md'
```

---

## 10. Scripts & Tooling

**Patterns:**

- Shell scripts in `scripts/` not referenced by any pipeline, justfile, package.json, Makefile, or README.
- `Makefile` targets nothing depends on (besides root targets).
- `justfile` recipes never invoked.
- One-off migration scripts left after the migration completed.

**Detection:**

```bash
# Find scripts and check for callers
fd -t f -e sh -e bash scripts/ | while read s; do
  name=$(basename "$s")
  count=$(rg -c "$name" -g '!scripts/*' --no-filename 2>/dev/null | awk -F: '{s+=$1} END {print s+0}')
  [ "$count" -eq 0 ] && echo "Possibly orphan script: $s"
done
```

---

## 11. Deployment Targets / Environments

**Patterns:**

- Staging environments / preview environments configured but unused.
- Build profiles / release configurations for platforms / regions you no longer ship to.
- Code-signing certificates / provisioning profiles for apps no longer distributed.
- DNS records pointing at decommissioned services.

These typically can't be detected from the repository alone — require operational data.

---

## Detection Strategy

1. **Inventory first.** Build a list of "what's declared" before checking "what's used." Most tools fail because they only have the latter.
2. **Static + dynamic.** Static reveals "no static reference exists." Production logs reveal "no actual traffic." The combination is what Meta's SCARF uses (see `safe-removal.md`).
3. **Allowlist runtime conventions.** Many env vars, config keys, and routes are read by frameworks via convention. Maintain an allowlist of known-conventional names.
4. **Track over time.** A route with no traffic _this week_ might be a quarterly reporting endpoint. Look at quarters of data before deleting.
