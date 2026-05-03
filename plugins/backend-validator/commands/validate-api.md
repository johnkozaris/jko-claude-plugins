---
description: Run Hurl-based API validation against a running backend, acquiring an OIDC token first if needed.
argument-hint: "[hurl-file-or-dir]"
allowed-tools:
  - Read
  - Edit
  - Write
  - Grep
  - Glob
  - Bash
user-invocable: true
---

# /validate-api

Drive Hurl-based REST validation. If `$TOKEN` isn't set in the environment, acquire one first via the project's token script.

## Steps

1. **Locate Hurl tests.** If the user passed a path argument, use that. Otherwise look for `**/tests/e2e/*.hurl` in the repo. If none exist, stop and ask the user if they want to scaffold a smoke test.

2. **Check for `$TOKEN`.** If already set in the environment, use it. Otherwise:
   - Look for `scripts/get-backend-token.sh` (the standard pattern this plugin documents)
   - If it exists, shell out to it and capture stdout as `TOKEN`
   - If missing, consult the `backend-validation` skill's token-acquisition pattern and either write the script or prompt the user to configure it

3. **Determine `base_url`** from the project's dev config. Respect `API_BASE_URL` if set.

4. **Confirm the backend is running** with a quick probe (e.g. a health endpoint). If it isn't, tell the user to start it before continuing.

5. **Run the tests.**

   ```bash
   hurl --test --variable token=$TOKEN --variable base_url=$BASE_URL <target>
   ```

6. **Parse output.** On failure, identify which assertion failed in which file, surface the relevant snippet, and suggest next steps (fix the test, fix the backend, or refresh the token if the failure smells like 401).

## Guardrails

- Never echo `$TOKEN` in plain text output — redact or show only the last 8 chars
- If a test hits a production URL, stop and confirm with the user first
- Don't auto-create `.hurl` files in random directories — place them under `<backend>/tests/e2e/` or confirm the location with the user

## When to defer to the skill

If the user asks a question that goes beyond "run these tests" — e.g. "why am I getting 401?", "how do I capture a response header?", "how do I retry eventually-consistent endpoints?" — invoke the `backend-validation` skill to get the full pattern reference.
