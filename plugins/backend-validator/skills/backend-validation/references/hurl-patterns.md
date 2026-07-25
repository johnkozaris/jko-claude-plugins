# Hurl validation patterns

Pass access tokens through Hurl's secret interface so reports and diagnostics
can redact them:

```bash
hurl --test \
  --secret token="$TOKEN" \
  --variable base_url="$API_BASE_URL" \
  tests/e2e/*.hurl
```

Use ordinary variables for environment URLs and fixture values. Use Hurl
captures for values created inside the scenario, such as a resource ID consumed
by a later request; do not duplicate OIDC login inside every Hurl file.

Use retry options only for an operation whose product contract is eventually
consistent. Tie retry count/interval to the expected bound and keep mutation
idempotent.

Parallel files need independent mutable state. Shared accounts, fixtures, or
database rows can make a parallel pass/fail nondeterministic.

JUnit/HTML reports may contain response bodies. Treat their artifact storage as
sensitive and confirm redaction before uploading.

Use current `hurl --help` and https://hurl.dev/docs/ for assertion and report
syntax rather than preserving a catalogue here.
