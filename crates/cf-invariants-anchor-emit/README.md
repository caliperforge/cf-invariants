# cf-invariants-anchor-emit

Fixture emission for the
[cf-invariants-anchor](https://github.com/caliperforge/cf-invariants-anchor)
workspace: turns candidate invariants into ready-to-run
Crucible-compatible `#[fuzz_fixture]` + `#[invariant_test]` source
files the harness loads directly. This crate does not rebuild the
harness — Crucible (and Trident, staged for Phase 1) own the LiteSVM
execution rails.

MSRV is pinned at Rust 1.79 via the workspace `rust-version`.

## License

Apache-2.0.

## Repository

Source, CI status, reference programs, scorecards, and roadmap:
[github.com/caliperforge/cf-invariants-anchor](https://github.com/caliperforge/cf-invariants-anchor).
