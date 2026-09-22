# Problem

The current ticket query contract is not expressive enough for focused discovery.

What exists now:

- the shared `memory-api` query AST only supports `Expr::And`, `Expr::Fts`, and exact/range field predicates
- ticket workflow surfaces still expose narrow bespoke filters instead of a common query object
- implicit ordering exists, but the API surface does not define how explicit ordering composes with default ranking

What is needed now:

- logical combinators such as `and`, `or`, and `not`
- deep field search over structured ticket metadata, not only flat equality predicates
- comparison operators such as `contains`, `gt`, `gte`, `lt`, `lte`, and clear range behavior
- first-class text-search clauses because substring and text search are common operator workflows
- ordering that stays close to Rust iterator/pipeline semantics where practical: selection narrows first, ordering applies after filtering, multi-key ordering composes lexicographically, and deterministic tie-breakers are always defined
- workflow surfaces (next, board, MCP, HTTP) that can consume the same query contract without inventing a second language

# Scope

1. Specify a canonical query model for ticket discovery across list, search, board, next, MCP, and HTTP.
2. Define the minimum operator set and the exact semantics for equality vs contains vs full-text search, deep field paths and dynamic field namespaces, boolean composition, scalar comparisons and range semantics, and null/missing-field handling.
3. Specify ordering behavior: default ordering for list/search/next/board, explicit order clauses and multi-key lexicographic chaining, deterministic stable tie-breaks, and where workflow-specific ranking remains authoritative versus where caller-specified order is allowed.
4. Define a transport-safe shape for CLI, MCP, and HTTP so the same query can be expressed without each adapter inventing new field names.
5. Document how the model should preserve a Rust-like iterator mental model while still providing a good operator UX.

# Acceptance Criteria

- Canonical spec defines the expressive ticket query language and ordering contract.
- The spec names the supported logical operators, comparison operators, deep-field path rules, and text-search behavior.
- The spec defines explicit ordering semantics and how they compose with current workflow ranking.
- The spec identifies which existing surfaces reuse the shared query model versus retain specialized defaults.
- The spec includes a validation matrix spanning parser tests, storage/search tests, CLI/MCP/HTTP parity tests, and workflow-surface regression cases.

# Resolution

Authored the canonical contract spec in the memory-api spec store:

- spec id: 08aa283e-34ee-47d4-83bc-4c4311a9c85f
- slug: ticket-query/expressive-query-and-ordering
- store path: memory-api/.spec/specs/08aa283e-34ee-47d4-83bc-4c4311a9c85f/

The spec locks: the select -> order -> truncate pipeline; the predicate AST extension (CompareOp = Eq/Contains/Gt/Gte/Lt/Lte/Range/Exists, FieldPath deep addressing); comparison token grammar (key:~, key:>, key:>=, key:<, key:<=, key:[a TO b], key:?); type-aware comparison (temporal/numeric/ordinal-enum/string); null/existence rules; explicit ordering (multi-key lexicographic, direction, deterministic id tie-break); and the composition rule that explicit order is authoritative on list/search but only a secondary tie-breaker after the authoritative workflow comparator on next/board/MCP/HTTP workflow surfaces. Transport-safe canonical string form defined for CLI/MCP/HTTP. Validation matrix defined across parser, storage/search, adapter parity, and workflow composition.

spec health 08aa283e reports 0 issues; spec is searchable in the memory-api store. Related specs linked: memory-api/model/query, memory-api/storage/search, ticket-api/workflow/best-next-ordering, ticket-api/workflow/blocker-trees-and-recently-unblocked-ordering.

# Likely Surfaces

- memory-api/.spec/
- memory-api/crates/memory-api/src/model/query.rs
- memory-api/crates/memory-api/src/storage/search.rs
- memory-api/crates/ticket-api/
- memory-api/tools/cli/ticket-cli/
- memory-api/tools/mcp/ticket-mcp/
- memory-api/tools/http/ticket-http/
