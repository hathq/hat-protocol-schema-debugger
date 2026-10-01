# hat-protocol-schema-debugger

Debug-only Hathq tool that inventories ecosystem `://` identifiers, classifies
them as protocol, schema, vocabulary, ledger, transport or resource references,
applies that projection to `zixcel-graph`, and exports a bounded JSON view for
Hatter Console.

The tool does not make a URI authoritative. It exposes the current state so
release work can see where protocol, schema and resource identifiers are mixed.

## Usage

```bash
cargo run -- scan \
  --ecosystem-root $WONDERLAND_ROOT/ecosystem/providers \
  --output generated/protocol-schema-graph.json
```

Optional durable graph output:

```bash
cargo run -- scan \
  --ecosystem-root $WONDERLAND_ROOT/ecosystem/providers \
  --graph-db /tmp/hatter-protocol-schema.redb \
  --output generated/protocol-schema-graph.json
```

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
