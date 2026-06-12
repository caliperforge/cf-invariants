# cf-invariants-anchor

AI invariant-author for [Crucible](https://github.com/asymmetric-research/crucible)
and [Trident](https://github.com/Ackee-Blockchain/trident) on Solana / Anchor.

Ingests an Anchor 0.30 / Codama-style IDL, proposes ranked invariant
candidates from a class library, and emits a ready-to-run
`#[fuzz_fixture]` + `#[invariant_test]` source file the harness can
load directly. This crate does not rebuild the harness — Crucible and
Trident already own the LiteSVM execution rails. The unclaimed
quadrant is invariant authoring, and that is what `cf-invariants-anchor`
ships.

## Install

```
cargo install cf-invariants-anchor-cli
```

The binary is named `cf-invariants-anchor`.

## Use

```
cf-invariants-anchor ingest  <idl.json>
cf-invariants-anchor suggest <idl.json> [--ai] [--audit-dir <dir>]
cf-invariants-anchor emit    <idl.json> [--target crucible|trident] [--candidate-index N]
```

`suggest --ai` runs the AI invariant-author flow against a pinned
prompt. Default builds use a deterministic `MockTransport` so CI stays
hermetic; the live Anthropic path compiles in only with the `live-ai`
feature flag plus `CF_INVARIANTS_ANCHOR_AI_LIVE=1` and
`ANTHROPIC_API_KEY` set at runtime.

## Classes shipped (Phase 0)

- `balance_conservation`
- `monotonic_accounting`
- `access_control`

## Toolchain

MSRV is pinned at Rust 1.79 via the workspace `rust-version`.

## License

Apache-2.0.

## Repository

Source, CI status, reference programs, scorecards, and roadmap:
[github.com/caliperforge/cf-invariants-anchor](https://github.com/caliperforge/cf-invariants-anchor).
