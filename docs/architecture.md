# cf-invariants-anchor — architecture

## One-line

Anchor IDL ingest → ranked invariant suggester (extensible class registry) → Crucible-compatible fuzz fixture emit → harness run → scorecard. Sidecar to Crucible v0.2.0 (primary) and Trident (Phase 1 secondary).

## Pipeline

```
  Anchor IDL JSON
         │
         ▼
  cf-invariants-anchor-idl   ─► ContractSurface { program_id, instructions, balance_fields }
         │
         ▼
  cf-invariants-anchor-suggest
    ClassRegistry::default()
      ├── BalanceConservation  (shipping)
      ├── MonotonicAccounting  (shipping)
      ├── AccessControl        (shipping)
      └── OracleFreshness      (Phase 2)
  cf-invariants-anchor-ai      (live OR mock)
      └── wraps the same candidates with InvariantSource::AiSuggested
         │
         ▼
  Vec<InvariantCandidate> (ranked, source-tagged)
         │
         ▼
  cf-invariants-anchor-emit
    ├── Target::Crucible    ─► #[fuzz_fixture] + #[invariant_test]
    └── Target::Trident     ─► Phase 1 stub
         │
         ▼
  fuzz/<program>/src/main.rs  (drop into your Anchor workspace)
         │
         ▼
  crucible run <program> <invariant_name> --release
         │
         ▼
  cf-invariants-anchor-report  ─► scorecard.{json,md}
```

## Crucible API surface targeted

**Empirically confirmed 2026-06-01** against the v0.2.0 tag (released
2026-05-26) by fetching `Cargo.toml`s and `src/` from the tag, *not* by
code-shape eyeballing. The upstream reference is
`examples/escrow/programs/escrow/src/lib.rs` + `examples/escrow/fuzz/escrow/src/main.rs`.
`anchor-lang = "1.0.1"` is verified directly against the v0.2.0
workspace `Cargo.toml`. The build/run claim itself (harness compiles
and behaves as expected) is proven by `.github/workflows/ci.yml`, not
by this document.

The shape cf-invariants-anchor's emit crate targets:

| Surface              | Real shape                                             |
|----------------------|--------------------------------------------------------|
| Crate name           | `crucible_fuzzer` (path dep `crucible-fuzzer`)         |
| Companion crate      | `crucible-test-context` (path dep)                     |
| Fixture macro        | `#[fuzz_fixture]` on `impl FixtureStruct`              |
| Invariant macro      | `#[invariant_test] fn invariant_xxx(&mut Fixture)`     |
| Action params        | `#[range(low..high)] param: T`                         |
| Assertion macros     | `fuzz_assert_eq!`, `fuzz_assert_lt!`, `fuzz_assert_le!`|
| Test context         | `TestContext::new()` with `add_program`, `program(id).call(ix).accounts(...).signers(...).send()`, `slot()`, `warp_to_slot(s)`, `read_anchor_account::<T>(&pda)` |
| CLI                  | `crucible init <program>`, `crucible run <program> <test_name> --release`, `crucible tmin`, `crucible show` |
| Feature wiring       | Each `#[invariant_test]` gets a corresponding `[features]` entry in fuzz `Cargo.toml`; **feature name must exactly match the function name** |

**Drift note vs. the original ticket:** the dispatch
(`T-cf-invariants-anchor-phase0-2026-06-01`) referenced an emit target
of `crucible::invariants!{…}`. That is not the v0.2.0 API. We
emit the real `#[fuzz_fixture]` + `#[invariant_test]` shape, validated
against the upstream escrow example. The ticket's other API
expectations (LiteSVM rails inherited, IDL-driven, `declare_fuzz_program!`
macro lineage) all hold.

## Emit-target abstraction

`cf-invariants-anchor-emit::Target` is a small enum (`Crucible`,
`Trident`) consumed by a single `render` entry point. The Trident
arm is a stub today; Phase 1 will replace it with a `FuzzData` + `#[init]`
rendering against the Trident manual-invariant shape documented at
[ackee.xyz/trident/docs](https://ackee.xyz/trident/docs/latest/).

## Invariant-class abstraction

`InvariantClass` is a trait with one method, `propose(&surface)`.
`ClassRegistry::phase0()` registers only `BalanceConservation`. Adding
a new class (Phase 1+) is one new file: implement the trait, register
it in `ClassRegistry::phase1()` (or downstream consumer code), and
the CLI / scorecard / emit pathways all pick it up automatically — the
ranking framework, source-tagging, and emit-hint plumbing are class-agnostic.

## AI-disclosure path

Default `suggest` runs the heuristic suggester (no AI call). Candidates
carry `InvariantSource::Heuristic { suggester_version }` and the
scorecard's AI-disclosure banner is dormant.

The `suggest --ai` path routes through `cf-invariants-anchor-ai`,
which wraps the same candidate shape with `InvariantSource::AiSuggested
{ model, prompt_version, timestamp_utc }` and writes a JSON audit-log
entry to `.cf-invariants-anchor/ai-log/<timestamp>.json` (timestamp,
model, prompt version, token counts, cost in USD, SHA-256 of the raw
response). Two transports sit behind the same trait:

- `MockTransport` (default): returns a canned response derived from
  the heuristic suggester. Used by CI and by anyone without an API
  key — proves the full provenance + audit-log pipeline without a
  live call.
- `LiveAnthropicTransport` (feature-gated): calls Anthropic Messages
  API with the pinned prompt at `prompts/invariant_suggestion_v1.txt`.
  Compiled only with `--features live-ai`; activated only when
  `CF_INVARIANTS_ANCHOR_AI_LIVE=1` AND `ANTHROPIC_API_KEY` is set.

The renderer rule is type-enforced: the AI-disclosure line cannot be
dropped whenever any `InvariantCandidate.source.is_ai_suggested()` is
part of the run.

See `docs/ai-disclosure.md`.
