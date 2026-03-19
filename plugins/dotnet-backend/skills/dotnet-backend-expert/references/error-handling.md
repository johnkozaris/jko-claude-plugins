# Error Handling

## Separate Expected and Unexpected Failures

Expected failures:

- validation errors
- not found
- conflict / concurrency business failures
- rule violations the caller can act on

Unexpected failures:

- code defects
- infrastructure outages
- serialization bugs
- timeouts or environmental faults the caller did not cause

Map expected failures intentionally. Let unexpected failures flow to a global exception boundary.

## API Edge Policy

At the host edge:

- enable `ProblemDetails`
- use a global exception handler (`UseExceptionHandler`, `IExceptionHandler`, or equivalent)
- keep one consistent RFC 7807 error shape
- sanitize detail text in production

## Domain and Application Outcomes

Do not throw framework exceptions from domain code. Prefer typed outcomes or domain-specific exceptions that the edge can map.

Examples:

- `OrderNotFound`
- `EmailAlreadyUsed`
- `InsufficientCredit`

The API boundary decides whether these become `404`, `409`, `400`, or another response.

## Logging

Log once with context at the boundary that owns the failure.

- include correlation and useful identifiers
- keep PII out of logs unless policy explicitly allows it
- avoid duplicate logging at every layer
- never swallow after logging

## Smells

| Smell | Signal | Fix |
|---|---|---|
| Swallowed exception | `catch` logs and returns success-ish result | rethrow or map deliberately |
| Business exceptions as flow control everywhere | normal invalid states use exceptions deep in core | use explicit outcomes where clearer |
| Inconsistent errors | each endpoint invents its own JSON shape | standardize on `ProblemDetails` |
| Detail leakage | stack traces or raw exception text in responses | sanitize at edge |
| 500 for known case | domain/application failure becomes generic server error | map explicitly |
