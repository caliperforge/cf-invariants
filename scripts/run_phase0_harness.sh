#!/usr/bin/env bash
# cf-invariants-anchor Phase 0 harness runner.
#
# Reproduces, on a Solana-toolchain-equipped host, the two Crucible runs
# that the CI workflow at `.github/workflows/ci.yml` runs on every push:
# clean variant should report 0 violations; planted variant should
# report at least 1 violation with a short counterexample.
#
# Requires on PATH:
#   - cargo (Rust stable, 1.79+ for our workspace; Crucible builds with
#     whatever the Anza platform-tools below carries)
#   - solana CLI (Anza installer, v2.1+ recommended) for `cargo-build-sbf`
#   - crucible CLI (`cargo install --path crates/crucible-fuzz-cli`
#     from an asymmetric-research/crucible @ v0.2.0 clone)
#
# Layout assumption: the upstream Crucible clone lives at
#   <repo-root>/experiments/crucible/
# (a sibling of cf-invariants-anchor under experiments/). That satisfies
# the `../../../../../crucible/...` path deps in
# `references/vault_ref{,_planted}/fuzz/vault_ref/Cargo.toml`. CI sets
# this up automatically; for local reproduction either clone Crucible to
# that path or vendor `crucible-fuzzer` + `crucible-test-context` and
# edit the fuzz Cargo.tomls.

set -euo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
TOOLS_VERSION="${SOLANA_TOOLS_VERSION:-v1.52}"
CRUCIBLE_TIMEOUT="${CRUCIBLE_TIMEOUT:-30}"

echo "==> cf-invariants-anchor workspace tests"
cargo test --manifest-path "$HERE/Cargo.toml" --workspace

# Helper: build a reference program, re-emit its invariant from the IDL,
# and run Crucible on both clean+planted variants.
#   $1 = ref name           (vault_ref, counter_ref, admin_ref)
#   $2 = program crate dir  (program-crate folder name)
#   $3 = idl filename       (basename under idls/)
#   $4 = emit candidate idx (0 for balance, 1 for monotonic/access)
#   $5 = invariant fn name  (Crucible --invariant)
run_pair() {
  local ref="$1" prog="$2" idl="$3" idx="$4" inv="$5"

  echo
  echo "==> cargo build-sbf ${ref} (clean)"
  cargo build-sbf \
      --tools-version "$TOOLS_VERSION" \
      --manifest-path "$HERE/references/${ref}/programs/${prog}/Cargo.toml"

  echo
  echo "==> cargo build-sbf ${ref} (planted)"
  cargo build-sbf \
      --tools-version "$TOOLS_VERSION" \
      --manifest-path "$HERE/references/${ref}_planted/programs/${prog}/Cargo.toml"

  echo
  echo "==> re-emit ${inv} from IDL (sanity)"
  cargo run --manifest-path "$HERE/Cargo.toml" \
      --bin cf-invariants-anchor --quiet -- \
      emit \
      "$HERE/references/${ref}/idls/${idl}" \
      --target crucible \
      --candidate-index "$idx" \
      --out "$HERE/references/${ref}/fuzz/${prog}/src/main.rs"
  cp "$HERE/references/${ref}/fuzz/${prog}/src/main.rs" \
     "$HERE/references/${ref}_planted/fuzz/${prog}/src/main.rs"

  echo
  echo "==> Crucible run on CLEAN ${ref} (expect 0 violations within ${CRUCIBLE_TIMEOUT}s)"
  ( cd "$HERE/references/${ref}/fuzz/${prog}" && \
    crucible run "${prog}" "${inv}" \
        --release --timeout "$CRUCIBLE_TIMEOUT" )

  echo
  echo "==> Crucible run on PLANTED ${ref} (expect >=1 violation)"
  ( cd "$HERE/references/${ref}_planted/fuzz/${prog}" && \
    crucible run "${prog}" "${inv}" \
        --release --timeout "$CRUCIBLE_TIMEOUT" || \
    echo "Crucible reported a violation (expected for the planted variant)." )
}

# Pair 1 — balance_conservation
run_pair vault_ref    vault_ref    vault_ref.json    0 invariant_amount_conservation
# Pair 2 — monotonic_accounting
run_pair counter_ref  counter_ref  counter_ref.json  1 invariant_lifetime_deposited_monotonic
# Pair 3 — access_control
run_pair admin_ref    admin_ref    admin_ref.json    1 invariant_withdraw_rejects_unauthorized

echo
echo "==> Done. Compare the runs to:"
for ref in vault_ref counter_ref admin_ref; do
  echo "    $HERE/findings/${ref}_clean/scorecard.expected.md"
  echo "    $HERE/findings/${ref}_planted/scorecard.expected.md"
done
echo "Real captured scorecards (from the CI run) live alongside, at"
echo "    $HERE/findings/<ref>_{clean,planted}/scorecard.md"
echo "once CI has produced them (see .github/workflows/ci.yml)."
