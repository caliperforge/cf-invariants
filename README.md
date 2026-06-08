# cf-invariants-anchor

[![ci](https://github.com/caliperforge/cf-invariants-anchor/actions/workflows/ci.yml/badge.svg)](https://github.com/caliperforge/cf-invariants-anchor/actions/workflows/ci.yml)

**The AI invariant-author for [Crucible](https://github.com/asymmetric-research/crucible) and [Trident](https://github.com/Ackee-Blockchain/trident) on Solana / Anchor.**

cf-invariants-anchor is a *sidecar* to the two open invariant-fuzzing
harnesses for Solana programs:

- **Crucible** (Asymmetric Research, MIT, LibAFL + LiteSVM, v0.2.0) — primary emit target.
- **Trident** (Ackee Blockchain, MIT) — secondary emit target (staged for Phase 1).

We **do not rebuild the harness**. Crucible and Trident already own
the LiteSVM execution rails and the IDL-driven program-fuzzing
plumbing. The unclaimed quadrant — and the only thing this crate
ships — is the **AI-suggested invariant author** that sits on top
of them: ingest an Anchor IDL, propose ranked candidate invariants
from a class library (balance conservation in Phase 0; monotonicity,
access-control, oracle-freshness on the roadmap), and emit a
ready-to-run `#[fuzz_fixture]` + `#[invariant_test]` source file
for Crucible.

This package is the Anchor sibling of the Cairo-targeted
[cf-invariants](../cf-invariants/) (Starknet / snforge) shipped by
the same operator.

---

## Status

**Phase 0 — working artifact, pre-1.0.**

> **Source of truth: the CI badge above.** GitHub Actions builds the
> workspace, builds all three reference program pairs via `cargo build-sbf`,
> and runs the Crucible v0.2.0 harness on every (class × clean/planted)
> cell on every push — asserting 0 violations on each clean variant
> and ≥1 violation on each planted variant. The captured scorecards
> from each run are uploaded as a CI artifact and committed into
> `findings/{vault_ref,counter_ref,admin_ref}_{clean,planted}/scorecard.md`
> once green. The `.expected.{json,md}` siblings remain as the authored
> reference; the unsuffixed files are the real captures.
>
> Pairs currently proven in CI:
> - `vault_ref` / `vault_ref_planted` — class `balance_conservation`
> - `counter_ref` / `counter_ref_planted` — class `monotonic_accounting`
> - `admin_ref` / `admin_ref_planted` — class `access_control`
>
> No "verified" claim in this repo is made by hand — if the badge is red,
> the artifact is not green. See
> [`findings/LIVE_VALIDATION_PENDING.md`](./findings/LIVE_VALIDATION_PENDING.md)
> for the per-class validation status.

What is live in this build:

- `cf-invariants-anchor ingest <idl>` — parses Anchor 0.30 / Codama-style
  IDL JSON into a typed contract surface (program id, instructions,
  scalar balance-bearing fields per stored account).
- `cf-invariants-anchor suggest <idl>` — produces a ranked list of
  candidate invariants from the heuristic suggester. **Three classes
  ship today**: `balance_conservation`, `monotonic_accounting`,
  `access_control`. The `InvariantClass` trait + `ClassRegistry` keep
  adding a fourth (oracle/freshness) a one-file change.
- `cf-invariants-anchor suggest <idl> --ai` — routes through
  `cf-invariants-anchor-ai` for AI-suggested candidates. Every returned
  candidate carries `InvariantSource::AiSuggested { model,
  prompt_version, timestamp_utc }`; a JSON audit-log entry is written
  to `.cf-invariants-anchor/ai-log/<timestamp>.json` (token counts,
  cost in USD, SHA-256 of response). Default transport is `MockTransport`
  (deterministic, no API key, CI-safe); `LiveAnthropicTransport`
  activates when the CLI is built with `--features live-ai` AND
  `CF_INVARIANTS_ANCHOR_AI_LIVE=1` AND `ANTHROPIC_API_KEY` set.
- `cf-invariants-anchor emit <idl> --target crucible` — renders a
  Crucible-compatible fuzz fixture for the selected candidate. Each
  class has its own emit shape: balance uses a fixture-side ledger +
  `fuzz_assert_eq!`; monotonic uses a `last_seen_*` snapshot + `fuzz_assert_le!`;
  access-control uses an attacker-keypair probe + sticky-flag assertion.
- `--target trident` — Phase-1 stub. Emit returns an explanatory
  placeholder so the CLI surface is reachable without the Trident
  rendering being wired.
- **Three reference contract pairs**, one per shipping class:
  - `references/vault_ref{,_planted}` — `balance_conservation`. The
    planted withdraw transfers `amount` lamports but decrements
    `vault.amount` by `amount-1`; the conservation invariant catches
    the drift in 1 withdraw.
  - `references/counter_ref{,_planted}` — `monotonic_accounting`. Adds
    a `lifetime_deposited: u64` ratchet field; the planted withdraw
    decrements it on every call, regressing the lifetime counter.
  - `references/admin_ref{,_planted}` — `access_control`. Clean
    withdraw enforces `seeds = [b"vault", depositor.key().as_ref()]`
    AND `has_one = depositor`; the planted variant drops both, so any
    signer can drain any vault PDA. The emitted attacker-probe
    fixture trips on the first successful unauthorized withdraw.
- **Scorecard renderer** with the AI-disclosure banner emitted
  whenever `ai_suggestions_included > 0`. The default reference run
  uses the heuristic source so the banner is dormant on those
  scorecards (the renderer pathway is exercised by tests); the
  `--ai` flag fires the banner.
- Workspace test suite (36 tests, `cargo test --workspace`).

## What it is not

- **Not a fork of Crucible or Trident.** Both ship as upstream
  dependencies; cf-invariants-anchor only authors invariants.
- **Not a formal-verification tool.** Randomized invariant search,
  not proofs.
- **Not a replacement for hand-authored invariants.** AI / heuristic
  candidates are always labeled `UNVERIFIED` until the contract
  author accepts them.

---

## Architecture

`cf-invariants-anchor` is a Cargo workspace of five Rust crates:

```
crates/
  cf-invariants-anchor-cli/      # binary
  cf-invariants-anchor-core/     # shared types: ContractSurface, InvariantCandidate, Scorecard
  cf-invariants-anchor-idl/      # Anchor IDL parser → ContractSurface
  cf-invariants-anchor-suggest/  # ClassRegistry + balance_conservation suggester
  cf-invariants-anchor-emit/     # Render to Crucible / Trident-stub source
  cf-invariants-anchor-report/   # Scorecard markdown + JSON renderer
references/
  vault_ref{,_planted}/          # balance_conservation pair
  counter_ref{,_planted}/        # monotonic_accounting pair
  admin_ref{,_planted}/          # access_control pair
findings/
  vault_ref_{clean,planted}/     # scorecard.expected.md + CI capture
  counter_ref_{clean,planted}/   # scorecard.expected.md + CI capture
  admin_ref_{clean,planted}/     # scorecard.expected.md + CI capture
docs/
  architecture.md                # design, emit-target abstraction, Crucible API record
  ai-disclosure.md               # AI involvement, disclosure path, audit log
scripts/
  run_phase0_harness.sh          # reproduce-from-clone driver
prompts/
  invariant_suggestion_v1.txt    # versioned prompt (Phase 1 AI path)
```

## Pinned toolchain

These are the versions CI builds against on every push (see
[`.github/workflows/ci.yml`](./.github/workflows/ci.yml)). All version
pins were empirically verified against the upstream tag's `Cargo.toml`,
not eyeballed from code shape:

- Rust **stable** (workspace MSRV: `1.79`).
- `anchor-lang` **1.0.1** — matches Crucible v0.2.0's workspace
  (`asymmetric-research/crucible @ v0.2.0` pins `anchor-lang = "1.0.1"`).
- Anza / Solana CLI **v2.1.21** for `cargo-build-sbf`.
- Solana platform-tools **v1.52** (Crucible v0.2.0 deps require
  edition2024 support; earlier platform-tools ship rustc 1.84 which
  cannot build them — passed as `--tools-version v1.52`).
- Upstream Crucible **v0.2.0** built from source in CI
  (`cargo install --path crates/crucible-fuzz-cli`).

The fuzz `Cargo.toml`s reference Crucible via path dep at
`../../../../../crucible/crates/crucible-fuzzer` — CI clones Crucible
to `<repo-root>/../crucible` so this resolves. For local reproduction
either clone Crucible to that path, or vendor the two crates and edit
the fuzz Cargo.tomls.

See [`.tool-versions`](./.tool-versions) (informational; CI is authoritative).

## Install

```sh
cargo install --path crates/cf-invariants-anchor-cli
```

## Quick start

```sh
# Ingest an Anchor IDL.
cf-invariants-anchor ingest references/vault_ref/idls/vault_ref.json

# Ask the suggester for ranked candidate invariants.
cf-invariants-anchor suggest references/vault_ref/idls/vault_ref.json

# Emit a Crucible #[fuzz_fixture] + #[invariant_test] file.
cf-invariants-anchor emit references/vault_ref/idls/vault_ref.json \
  --target crucible \
  --out references/vault_ref/fuzz/vault_ref/src/main.rs
```

## End-to-end Phase 0 demo

CI runs exactly this every push; local reproduction is optional. See
[`scripts/run_phase0_harness.sh`](./scripts/run_phase0_harness.sh) for
the canonical driver.

```sh
# 1. Build the clean reference program + its planted twin (SBPF).
cargo build-sbf --tools-version v1.52 \
  --manifest-path references/vault_ref/programs/vault_ref/Cargo.toml
cargo build-sbf --tools-version v1.52 \
  --manifest-path references/vault_ref_planted/programs/vault_ref/Cargo.toml

# 2. Emit the conservation invariant (already pre-emitted in this repo).
cf-invariants-anchor emit references/vault_ref/idls/vault_ref.json \
  --target crucible \
  --out references/vault_ref/fuzz/vault_ref/src/main.rs
cp references/vault_ref/fuzz/vault_ref/src/main.rs \
   references/vault_ref_planted/fuzz/vault_ref/src/main.rs

# 3. Run Crucible against both variants (timeout small enough for CI;
#    the planted bug's minimal counterexample is 2 actions and fires fast).
(cd references/vault_ref/fuzz/vault_ref && \
  crucible run vault_ref invariant_amount_conservation --release --timeout 30)
(cd references/vault_ref_planted/fuzz/vault_ref && \
  crucible run vault_ref invariant_amount_conservation --release --timeout 30)
```

The CI workflow captures the real output of these runs into
`findings/vault_ref_{clean,planted}/scorecard.md` and uploads them as
the `crucible-scorecards` artifact. The `scorecard.expected.{json,md}`
siblings remain as authored reference for diffing.

## Roadmap

| Phase | Surface |
|-------|---------|
| **0** | IDL ingest → ranked balance-conservation candidates → Crucible emit → scorecard renderer. Heuristic suggester only. ✅ shipped. |
| **1** | AI-suggested invariants live (Anthropic Claude Sonnet path: MockTransport default, LiveAnthropicTransport behind `--features live-ai` + env). Monotonic + access-control classes added to the suggester + emit. ✅ shipped. |
| **2 (this build)** | `counter_ref` (monotonicity) + `admin_ref` (access control) reference pairs with CI Crucible proof — each new class catches its planted bug under `clean=0 / planted≥1`. ✅ in-tree; CI-green gated on the next push. |
| 3 | Trident emit target. Oracle-freshness class. CI scorecard-drift early warning. Shrinking. Multi-account-state surface (full account-mutability-set analysis instead of name-heuristic). |

## Reporting issues, security contact

Open an issue on the GitHub repository, or contact
[team@caliperforge.com](mailto:team@caliperforge.com).

For sensitive security disclosures touching cf-invariants-anchor itself,
contact [michael@caliperforge.com](mailto:michael@caliperforge.com) directly.

## License

Apache-2.0. See `LICENSE`.

---

cf-invariants-anchor is operated by Michael Moffett under the CaliperForge banner. CaliperForge is a sole-operator engineering studio.

Built with AI assistance. Authored and reviewed by Michael Moffett, operator at CaliperForge. Full policy at [caliperforge.com/ai-disclosure](https://caliperforge.com/ai-disclosure). See [`docs/ai-disclosure.md`](./docs/ai-disclosure.md) for the in-repo detail on what the AI module does, the audit-log path, and how to disable it.

[caliperforge.com](https://caliperforge.com)
