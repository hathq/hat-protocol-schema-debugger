# hat-protocol-schema-debugger

Find protocol and schema references in a selected source area and inspect how they relate.

## What you can do

- Classify identifiers and flag ambiguous contract paths.
- Export a bounded diagnostic graph for review.

## Current scope

This is a debugging projection. Its labels and graph do not create authoritative product state.

Package distribution is not activated by this documentation. Use the checked-in source and the declared dependency versions; published availability must be verified separately.

## Getting started

Install Rust 1.97 or newer and make the declared dependencies available. Resolve declared third-party dependencies from public registries and source-only managed components from their fixed public Git revision. No private registry or adjacent checkout is part of the supported OSS source path. Run from this repository:

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

Source selection is configured in `config/scan-policy.json`. Use `--scan-policy`
or `PROTOCOL_SCHEMA_SCAN_POLICY` to select another policy file. Missing or
invalid policies stop scanning before a durable graph is opened.

The shipped policy excludes migration inputs, local state, registration data, generated
artifacts, and symbolic links. An excluded directory cannot be used as a scan
root. These private inputs are never part of the diagnostic source graph.

The generated JSON is intentionally UI-shaped. Hatter Console reads it as a
debug projection and never treats it as canonical product state.

## Documentation and source

[Interface reference](docs/interface-reference.md)

[Usage guide](docs/getting-started.md)

[Implementation and public interfaces](src) · [Contributing](CONTRIBUTING.md) · [Security reporting](SECURITY.md) · [License](LICENSE) · [Attribution notices](NOTICE)

[OSS responsibility and source completion](docs/oss-source-contract.md) defines the supported package boundary and required acceptance. It does not declare unexecuted source, distribution or production checks complete.
