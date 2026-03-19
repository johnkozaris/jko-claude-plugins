# Data Access

## Default Data Access Strategy

Use EF Core as the default for most backend services. Reach for Dapper or raw SQL only when you can name a concrete need:

- hot-path performance
- specialized SQL features
- bulk operations or reporting queries that EF Core makes awkward

## DbContext Rules

- keep `DbContext` scoped
- never share a context across threads
- use `IDbContextFactory<T>` in background workers or multi-unit-of-work flows
- keep EF Core in infrastructure or at least outside the pure domain model

## Read Patterns

Prefer projections and no-tracking for read-heavy paths:

- `AsNoTracking()` by default for read-only work
- project to DTOs or read models
- paginate large sets
- avoid loading wide graphs casually

## Write Patterns

- keep a clear use-case boundary for writes
- let one `SaveChanges` be the normal transaction boundary
- use explicit transactions only when a single use case truly spans multiple saves or resources
- keep transaction scope short

## Repository Tradeoffs

Repositories are useful when they:

- protect a domain boundary
- hide persistence complexity
- provide a stable seam for aggregates or complex stores

Repositories are noise when they:

- only mirror `DbSet`/`DbContext`
- force every query through generic CRUD wrappers
- explode into include/filter/sort overload soup

## Migrations

- keep migrations in source control
- review schema changes like code
- do not rely on casual automatic migration at startup in production
- keep database evolution explicit in deployment workflows

## Query Design Smells

- endpoint returns tracked entities directly
- read-only endpoint tracks large graphs unnecessarily
- repository exposes too many ad hoc query knobs
- one write request does multiple broad `SaveChanges` calls with no clear boundary
- background service reuses request-scoped `DbContext`

## Review Questions

- Is the transaction boundary aligned with a real use case?
- Would direct `DbContext` be simpler here than a repository?
- Is the query returning more rows or columns than the caller needs?
- Is persistence complexity leaking into application or domain code?
