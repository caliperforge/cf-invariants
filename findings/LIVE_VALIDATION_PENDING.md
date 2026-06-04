# cf-invariants-anchor — live validation status

**Updated 2026-06-02 (Phase-2 dispatch):** the two new invariant
classes (`monotonic_accounting`, `access_control`) shipped in Phase 1
were coded-but-unproven — only `balance_conservation` was CI-green via
the `vault_ref` pair. This dispatch closes that gap by adding a
`counter_ref` pair (monotonicity) and an `admin_ref` pair (access
control) and wiring both into CI alongside `vault_ref`. The artifact
is grant-competitive once
[![ci](https://github.com/caliperforge/cf-invariants-anchor/actions/workflows/ci.yml/badge.svg)](https://github.com/caliperforge/cf-invariants-anchor/actions/workflows/ci.yml)
is green on `main` across **all three** classes.

## What changed in this dispatch

1. **Two new reference program pairs.**
   - `references/counter_ref/` (clean) + `references/counter_ref_planted/`
     — Anchor vault with a `lifetime_deposited: u64` ratchet. The
     planted twin's `withdraw` decrements `lifetime_deposited` by
     `amount` (the field is a lifetime cumulative counter; a correct
     implementation never decreases it). The emitted
     `invariant_lifetime_deposited_monotonic` fixture detects the
     regression in 1 withdraw call.
   - `references/admin_ref/` (clean) + `references/admin_ref_planted/`
     — Anchor vault with depositor-gated withdraw. The clean variant
     enforces BOTH the PDA seeds derivation
     (`seeds = [b"vault", depositor.key().as_ref()]`) AND
     `has_one = depositor`. The planted twin drops both constraints,
     accepting any signer as `depositor`. The emitted
     `invariant_withdraw_rejects_unauthorized` fixture probes with a
     freshly-minted attacker `Keypair` and trips a sticky flag on the
     first successful unauthorized withdraw.

2. **CI extended** (`.github/workflows/ci.yml`):
   - 4 new `cargo build-sbf` steps (counter_ref + admin_ref, clean+planted).
   - 4 new Crucible runs, asserting clean=0 / planted≥1 per class.
   - 4 new ANSI-stripped scorecard captures under
     `findings/{counter_ref,admin_ref}_{clean,planted}/scorecard.md`,
     uploaded to the same `crucible-scorecards` artifact.
   - Harness job `timeout-minutes` raised from 75 → 120 to fit the
     three pairs end-to-end on a free-tier GitHub-hosted runner.

3. **Local-reproduction script** (`scripts/run_phase0_harness.sh`)
   refactored to a `run_pair()` helper, then called once per class.

## Resolution

Each class is marked **VALIDATED** once the CI run URL on `main`
is green for the corresponding pair:

| Class                  | Pair                  | Clean step                          | Planted step                         | Status      |
|------------------------|-----------------------|-------------------------------------|--------------------------------------|-------------|
| balance_conservation   | vault_ref             | `vault_ref_clean`                   | `vault_ref_planted`                  | already green (9eb3e88) |
| monotonic_accounting   | counter_ref           | `counter_ref_clean`                 | `counter_ref_planted`                | PENDING first green CI  |
| access_control         | admin_ref             | `admin_ref_clean`                   | `admin_ref_planted`                  | PENDING first green CI  |
| balance_conservation (Kamino-class) | kamino_lending_ref | `kamino_lending_ref_clean` | `kamino_lending_ref_planted` | PENDING first green CI (Phase-3 dispatch 2026-06-02) |

This file is overwritten with the run URLs + scorecard counts once CI
is green across all three classes; until then, the artifact is NOT
grant-competitive and is NOT ready to point at a real lending protocol.

## Standing rule encoded in this repo

> **"Verified" = actually built and run (in CI or locally), never code-shape
> eyeballing. No artifact is publishable until CI is green.**

## Local reproduction is optional

```bash
# 1. Clone Crucible at the v0.2.0 tag, as a sibling of cf-invariants-anchor.
git clone --branch v0.2.0 https://github.com/asymmetric-research/crucible.git ../crucible

# 2. Install crucible CLI.
cargo install --path ../crucible/crates/crucible-fuzz-cli --locked

# 3. Drive the full three-pair harness.
./scripts/run_phase0_harness.sh
```
