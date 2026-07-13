# Multi-seed reachability certification (cf-invariants-anchor)

## Status

**Uncovered: 0 of 4 planted classes certified today.** Framework files
land in this commit; the deterministic regression bins each planted
crate needs are queued as a follow-up spike per crypto-contributor
`T-reachability-solana-anchor-regression-bins-spike-2026-07-13`.

The base planted CI (`clean-passes` + `planted-twin-detects` in
`.github/workflows/ci.yml`) continues to run the Crucible fuzz leg
per commit. Reachability adds an orthogonal deterministic-regression
receipt on top; it does not replace the fuzz leg.

## What Shape A is

Shape A per crypto-contributor design proposal
`D-solana-reachability-leg-shape-2026-07-13` (Director-ratified):
each planted crate ships a `src/bin/regression.rs` that reads
`REACHABILITY_SEED`, drives the fixture actions via `StdRng`, and
prints the class-specific `INVARIANT VIOLATED <name>` marker on
divergence. `ci/reachability_leg.sh` iterates the 16-seed canonical
set (`ci/reachability_seeds.txt`, byte-identical to
`caliperforge/crypto-contributor:scripts/reachability/seeds.txt` and
to the sibling `caliperforge/solana-invariant-atlas` and
`caliperforge/soroban-invariant-atlas` copies) and requires rc!=0 +
marker on all 16 (fail-on-any-clean-seed does-not-merge).

Reference implementation:
`caliperforge/solana-invariant-atlas:references/collateral_mint_ref_planted/fuzz/collateral_mint_ref/src/bin/regression.rs`
(the C-A2 collateral_authority landing that shipped 16/16 on the
Shape A design; see the atlas's own `docs/reachability.md`).

## Planted classes in scope (need regression bins)

Per the crypto-contributor Phase-2c feasibility report
(`T-reachability-anchor-rust-solana-2026-07-13`):

| planted crate | invariant | class | regression bin status |
| --- | --- | --- | --- |
| `counter_ref_planted` | `invariant_lifetime_deposited_monotonic` | monotonic_accounting | absent (spike required) |
| `vault_ref_planted` | balance_conservation | balance_conservation | absent (spike required) |
| `kamino_lending_ref_planted` | (TBD per authored invariant) | (TBD) | absent (spike required) |
| `admin_ref_planted` | access_control | access_control | absent (spike required) |

None of the four planted crates today ships `src/bin/regression.rs`
under `references/<name>_planted/fuzz/<name>/src/bin/`. The Crucible
fuzz harness lives at `src/main.rs` in each; the seeded deterministic
regression is a sibling bin that Shape A introduces.

## Spike scope (queued)

Per planted class, the spike work adds:

1. `Cargo.toml` `[[bin]]` entry for `regression`.
2. `Cargo.toml` `[dependencies]` addition: `rand = "0.8"`.
3. `src/bin/regression.rs`: `parse_seed_env()` +
   `keypair_from_rng(rng)` scaffolding (copy verbatim from the atlas
   reference implementation), followed by the class-specific
   deterministic action sequence:
   - counter_ref: initialize → deposit → withdraw → assert
     `lifetime_deposited` did not decrease.
   - vault_ref: initialize → deposit → transfer_out → assert
     Σ balances == expected total.
   - kamino_lending_ref: (TBD per authored invariant).
   - admin_ref: initialize → non-admin call → assert access refused.
4. When `REACHABILITY_SEED` is absent, fallback to fixed values so
   normal `cargo run --release --bin regression` remains developer-
   friendly. The env-var-absent path is what CI's existing
   `cargo test` legs consume.

Each bin's action sequence is class-specific; correctness requires
reading the fixture's `#[fuzz_fixture]` block and the underlying
Anchor program to pick the right instruction order + account setup.
Not landable as a batch scaffold; needs per-class authorship.

## What lands today

- `ci/reachability_seeds.txt` — canonical 16-seed set, byte-identical
  to the sibling repo copies.
- `docs/reachability.md` — this file: shape, uncovered status,
  spike scope.

No workflow changes; no README verdict block claiming certification.
The `reachability` job lands with the spike PR once the four
regression bins are authored.

## Merge-gate rule (target)

Once regression bins exist per planted crate + the `reachability` job
lands, no new planted twin merges to `main` unless the leg exits
green (fail-on-all-N). k/N per-class certification numbers move into
the top-level README verdict block at that point.

## Reuse

The canonical scripts this leg mirrors live at
`caliperforge/crypto-contributor:scripts/reachability/` and the shape
matches `caliperforge/soroban-invariant-atlas:ci/reachability_leg.sh`
and `caliperforge/solana-invariant-atlas:ci/reachability_leg.sh`.
Copy from the Solana atlas as the closest per-ecosystem reference.
