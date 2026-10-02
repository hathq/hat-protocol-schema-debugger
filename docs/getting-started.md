# Using hat-protocol-schema-debugger

Find protocol and schema references in a selected source area and inspect how they relate.

## Before you start

This is a debugging projection. Its labels and graph do not create authoritative product state.

## First steps

Run from the repository root:

```sh
cargo test --locked
```

## How to assess the result

- Classify identifiers and flag ambiguous contract paths.
- Export a bounded diagnostic graph for review.

A passing source-level check establishes only what that check observes. Keep missing configuration, unavailable services and unverified deployment paths visible.

## Continue reading

[Repository overview](../README.md)
