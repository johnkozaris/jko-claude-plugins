Audit this project's Claude Code setup against best practices. Check each of these areas and report findings with specific, actionable fixes:

1. **CLAUDE.md** — Does it exist? Is it < 200 lines? Does it have: project overview, tech stack, build/test commands, architecture, conventions, gotchas? Is it bloated with obvious information?

2. **Skills** — Are there skills in `.claude/skills/` or `skills/`? Do they follow folder structure (SKILL.md + references/ + scripts/)? Do they have gotchas sections? Are descriptions written as triggers, not summaries?

3. **Verification** — Is there a way to verify the agent's output? Test suites, linter configs, verification scripts? Can Claude run tests after making changes?

4. **Prompt Caching** — Are tools stable across sessions (not added/removed dynamically)? Is static content before dynamic content? Are state updates in messages, not system prompt edits?

5. **Permissions** — Is there a `.claude/settings.json` with pre-approved safe commands? Or is the project using `--dangerously-skip-permissions`?

6. **Hooks** — Are there PostToolUse hooks for formatting after edits? PreToolUse hooks for safety gates?

7. **Project Structure** — Is the codebase organized for agent readability? Are entry points clear? Is the directory structure logical?

Present findings as a checklist with pass/fail and specific recommendations for each item.
