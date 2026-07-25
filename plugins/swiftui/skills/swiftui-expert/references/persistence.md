# Persistence decisions

Choose persistence from the product's data shape, query needs, migration risk,
sync topology, deployment targets, and team expertise--not from which framework
has the newest syntax.

SwiftData can fit Apple-platform applications with compatible targets and a
manageable model. Core Data remains legitimate for mature stores and CloudKit
topologies it supports well. GRDB or another SQLite layer fits products that
benefit from explicit SQL and deterministic migrations. Verify the current
capabilities of any newer wrapper before recommending it.

Ship a migration strategy with the first production schema. Test opening a copy
of the previous store, failed migration recovery, destructive reset policy, and
large real datasets. A model that works only from a clean install is not
production-ready persistence.

Known SwiftData traps to verify against the target SDK:

- If staged migration may be needed later, wrap the first shipped model in a
  `VersionedSchema`. A store created from an unknown model version can require a
  bridge release before staged migration becomes possible.
- Under the currently documented SwiftData CloudKit integration, non-optional
  attributes require defaults, relationships must be optional and have
  inverses, and uniqueness constraints are unsupported. Verify the target SDK
  for changes, then test real container initialization and sync; local-only
  success does not prove CloudKit validity.

Keep database work out of rendering and avoid accidental unbounded fetches.
Define ownership for contexts/connections and cancellation behavior around app
suspension.

Store small secrets in Keychain. UserDefaults is preference storage, not secret
storage or a database. Larger sensitive data needs appropriate file/database
protection and a lifecycle policy.
