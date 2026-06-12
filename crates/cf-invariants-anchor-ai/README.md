# cf-invariants-anchor-ai

Anthropic-backed AI invariant suggester for the
[cf-invariants-anchor](https://github.com/caliperforge/cf-invariants-anchor)
workspace: drives the AI invariant-author flow against a pinned prompt.
Default builds use a deterministic `MockTransport` so CI stays hermetic;
the live Anthropic path compiles in only with the `live` feature flag
plus `CF_INVARIANTS_ANCHOR_AI_LIVE=1` and `ANTHROPIC_API_KEY` set at
runtime.

MSRV is pinned at Rust 1.79 via the workspace `rust-version`.

## License

Apache-2.0.

## Repository

Source, CI status, reference programs, scorecards, and roadmap:
[github.com/caliperforge/cf-invariants-anchor](https://github.com/caliperforge/cf-invariants-anchor).
