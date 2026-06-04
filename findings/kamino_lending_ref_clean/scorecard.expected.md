# cf-invariants-anchor scorecard — kamino_lending_ref (CLEAN)

Faithful Kamino-class lending vault reference. Class: `balance_conservation`.
Invariant: `invariant_total_assets_conservation` on `LendingVault.total_assets`.

## Summary

- Invariants total: **1**
- Invariants violated: **0**
- AI-suggested invariants in this run: **0**
- Runtime: **1800 ms** (CI timeout `${CRUCIBLE_TIMEOUT}` = 30s)
- Crucible version: `0.2.0`

This is the AUTHORED EXPECTATION. The unsuffixed `scorecard.md` sibling is
captured by `.github/workflows/ci.yml` from the real Crucible run on
every push; if that capture ever diverges from `0` violations on the
clean variant, the invariant is wrong or the reference has a real bug.

---

cf-invariants-anchor — Apache-2.0. Operated by Michael Moffett — michael@caliperforge.com — team@caliperforge.com.
