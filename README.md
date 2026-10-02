# hat-protocol-schema-debugger

Find protocol and schema references in a selected source area and inspect how they relate.

## What you can do

- Classify identifiers and flag ambiguous contract paths.
- Export a bounded diagnostic graph for review.

## Current scope

This is a debugging projection. Its labels and graph do not create authoritative product state.

Package distribution is not activated by this documentation. Use the checked-in source and the declared dependency versions; published availability must be verified separately.

## Getting started

Install Rust 1.97 or newer and make the declared dependencies available. Use the configured private registry when a dependency is not distributed publicly. Run from this repository:

```sh
cargo test --locked
```

## Examples and interface details

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

## Documentation and source

[Interface reference](docs/interface-reference.md)

[Usage guide](docs/getting-started.md)

[Implementation and public interfaces](src) · [Contributing](CONTRIBUTING.md) · [Security reporting](SECURITY.md) · [License](LICENSE) · [Attribution notices](NOTICE)
