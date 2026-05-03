# Hurl Patterns

Everything below assumes `--variable base_url=<env>` and `--variable token=<jwt>` are passed at the CLI. Hurl files stay identical across environments.

## Minimum viable test

```hurl
GET {{base_url}}/health
HTTP 200
[Asserts]
jsonpath "$.status" == "ok"
```

Run: `hurl --test smoke.hurl` — exit 0 on success, non-zero on any assertion failure.

## Captures (auth chain inside one file)

```hurl
POST {{base_url}}/auth
{"email": "{{email}}", "password": "{{password}}"}
HTTP 200
[Captures]
token: jsonpath "$.access_token"

GET {{base_url}}/me
Authorization: Bearer {{token}}
HTTP 200
```

Captures survive only inside the current run. For tokens that should persist, acquire out-of-band (oauth2c) and pass via `--variable token=$TOKEN`.

## Assertions

```hurl
HTTP 200
[Asserts]
jsonpath "$.id" isString
jsonpath "$.items" count > 0
jsonpath "$.total" >= 5
jsonpath "$.email" matches /.+@.+/
header "X-Request-Id" exists
body contains "success"
duration < 500
```

Full list: https://hurl.dev/docs/asserting-response.html

## Retries (for eventually-consistent APIs)

```hurl
GET {{base_url}}/jobs/{{job_id}}
[Options]
retry: 10
retry-interval: 500
HTTP 200
[Asserts]
jsonpath "$.status" == "completed"
```

Hurl will retry the request up to 10 times with 500ms between attempts until assertions pass.

## Error bodies

```hurl
POST {{base_url}}/users
{"email": "invalid"}
HTTP 400
[Asserts]
jsonpath "$.errors[0].field" == "email"
jsonpath "$.errors[0].code" == "invalid_format"
```

## Parallel runs

```bash
hurl --test --parallel --jobs 4 tests/e2e/*.hurl
```

Each `.hurl` runs in its own worker. Good for independent test suites; bad for suites that share mutable state (e.g. same DB fixture).

## Output for CI

```bash
hurl --test --report-junit report.xml tests/e2e/*.hurl
hurl --test --report-html out/ tests/e2e/*.hurl
```

JUnit XML integrates with GitHub Actions, GitLab, Jenkins. HTML reports include response bodies and timings for debugging flaky tests.

## Parameterized data

```hurl
POST {{base_url}}/feedback
Content-Type: application/json
{
  "rating": {{rating}},
  "comment": "{{comment}}"
}
HTTP 201
```

Run: `hurl --test --variable rating=5 --variable comment="Great" smoke.hurl`

For many rows, use a data-file runner:

```bash
jq -c '.[]' fixtures.json | while read row; do
  RATING=$(echo "$row" | jq -r .rating)
  COMMENT=$(echo "$row" | jq -r .comment)
  hurl --test --variable rating=$RATING --variable comment="$COMMENT" feedback.hurl
done
```

## GraphQL

Hurl has first-class GraphQL support:

```hurl
POST {{base_url}}/graphql
Authorization: Bearer {{token}}
```
```graphql
query GetUser($id: ID!) {
  user(id: $id) { id name email }
}
```
```json
{"id": "{{user_id}}"}
```
```hurl
HTTP 200
[Asserts]
jsonpath "$.data.user.id" == "{{user_id}}"
jsonpath "$.errors" not exists
```
