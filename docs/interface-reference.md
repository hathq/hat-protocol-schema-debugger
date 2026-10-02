# hat-protocol-schema-debugger interface reference

Use the [usage guide](getting-started.md) for the first steps. This reference preserves the current interface details and operational limits. Run command examples from the repository root, after preparing the exact declared dependencies and registered configuration.

## Usage

Optional durable graph output:

The generated JSON is intentionally UI-shaped. Hatter Console reads it as a
debug projection and never treats it as canonical product state.

## Classification rule

- `protocol`: a communication, operation or exchange contract.
- `schema`: one message, record, result or payload shape.
- `vocabulary`: a meaning term, catalog or lexical item.
- `ledger`: a provenance or history ledger.
- `transport`: an external transport such as WebSocket or Cargo sparse HTTP.
- `resource`: repository, catalog, workspace, secret, evidence or other target.

Compound path segments containing `-` are reported as warnings only for
`hathq://` contract paths, because Hatter's shared contract policy expects each
owned segment to carry one machine-readable concept. Provider and resource
schemes remain opaque and are not rewritten by Hatter.

The 0.10.0 HatSpec contract namespace is `hathq://hat/...`; the first path
segment identifies `vocabulary`, `schema`, or `protocol`. Hatter-local
`hathq://hatter/...` identifiers are reported as `hatter-local-namespace` and
must not be published as shared contracts. A possible future `hat://hathq/...`
namespace is a breaking migration and is intentionally not treated as an
alias. Generated reports are excluded from subsequent scans to keep the
projection deterministic.
