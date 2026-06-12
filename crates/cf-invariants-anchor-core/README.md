# cf-invariants-anchor-core

Shared types for the [cf-invariants-anchor](https://github.com/caliperforge/cf-invariants-anchor)
workspace: the parsed contract surface, candidate invariants, and the
scorecard model. Every other `cf-invariants-anchor-*` crate depends on
this one; it carries no I/O and no harness logic — Crucible and Trident
own the execution rails.

MSRV is pinned at Rust 1.79 via the workspace `rust-version`.

## License

Apache-2.0.

## Repository

Source, CI status, reference programs, scorecards, and roadmap:
[github.com/caliperforge/cf-invariants-anchor](https://github.com/caliperforge/cf-invariants-anchor).
