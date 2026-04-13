# The Nine Skill Categories

> Anthropic uses hundreds of skills in active development across these categories.

## 1. Library & API Reference

Skills explaining how to correctly use a library, CLI, or SDK. Focus on edge cases and footguns Claude doesn't know, not obvious documentation.

**Examples:**
- `billing-lib` — Internal billing library with edge cases
- `internal-platform-cli` — Every subcommand with examples
- `frontend-design` — Design system patterns (built by iterating with customers)

**Key:** Include reference code snippets and a list of gotchas.

## 2. Product Verification

Skills describing how to test or verify output. Often paired with Playwright, tmux, or browser tools.

**Examples:**
- `signup-flow-driver` — Runs signup → email verify → onboarding in headless browser
- `checkout-verifier` — Drives checkout with Stripe test cards, verifies invoice state
- `tmux-cli-driver` — Interactive CLI testing needing a TTY

**Key:** Worth having an engineer spend a week making these excellent. Include verification scripts.

## 3. Data Fetching & Analysis

Skills connecting to data and monitoring stacks. Include credentials, dashboard IDs, common workflows.

**Examples:**
- `funnel-query` — Which events to join for signup → activation → paid
- `cohort-compare` — Compare retention, flag statistically significant deltas
- `grafana` — Datasource UIDs, cluster names, problem → dashboard lookup table

**Key:** Include helper functions to fetch data. Claude composes them for complex analysis.

## 4. Business Process & Team Automation

Skills automating repetitive workflows into one command. Save execution logs for consistency.

**Examples:**
- `standup-post` — Aggregates ticket tracker + GitHub + Slack → formatted standup
- `create-ticket` — Enforces schema, post-creation workflow (ping reviewer, link in Slack)
- `weekly-recap` — Merged PRs + closed tickets + deploys → formatted recap

**Key:** Log previous executions so Claude stays consistent and reflects on past runs.

## 5. Code Scaffolding & Templates

Skills generating boilerplate for specific codebase patterns. Combine template structures with natural language.

**Examples:**
- `new-workflow` — Scaffolds service/handler with annotations
- `new-migration` — Migration file template plus common gotchas
- `create-app` — New internal app with auth, logging, deploy config

**Key:** Useful when scaffolding has natural language requirements beyond pure code.

## 6. Code Quality & Review

Skills enforcing code standards and facilitating review. Run as hooks or in GitHub Actions.

**Examples:**
- `adversarial-review` — Spawns fresh-eyes subagent, critiques, iterates until findings degrade to nitpicks
- `code-style` — Enforces styles Claude doesn't do well by default
- `testing-practices` — How to write tests and what to test

**Key:** Can be automated via hooks. Consider deterministic scripts for maximum robustness.

## 7. CI/CD & Deployment

Skills managing code fetching, pushing, deployment with testing and rollback.

**Examples:**
- `babysit-pr` — Monitors PR → retries flaky CI → resolves merge conflicts → auto-merge
- `deploy-service` — Build → smoke test → gradual rollout with error-rate comparison → auto-rollback
- `cherry-pick-prod` — Isolated worktree → cherry-pick → conflict resolution → PR with template

**Key:** Reference other skills to collect data. Chain operations with verification at each step.

## 8. Runbooks

Skills that take a symptom and produce a structured investigation report.

**Examples:**
- `service-debugging` — Maps symptoms → tools → query patterns for high-traffic services
- `oncall-runner` — Fetches alert → checks usual suspects → formats findings
- `log-correlator` — Given request ID, pulls matching logs from every system

**Key:** Multi-tool workflows. Start from symptoms (Slack thread, alert, error), walk through investigation.

## 9. Infrastructure Operations

Skills performing routine maintenance with safety guardrails for destructive actions.

**Examples:**
- `resource-orphans` — Finds orphaned pods/volumes → Slack → soak period → confirm → cleanup
- `dependency-management` — Org's dependency approval workflow
- `cost-investigation` — "Why did our bill spike?" with specific query patterns

**Key:** Safety guardrails are critical. Include confirmation steps for destructive operations.

## Categorization Tips

- The best skills fit cleanly into ONE category
- Confusing skills straddle several — try to decompose them
- Use this list to find skill gaps in your organization
- Not all categories apply to every team
