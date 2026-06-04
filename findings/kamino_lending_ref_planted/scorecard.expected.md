# cf-invariants-anchor scorecard — kamino_lending_ref (PLANTED)

Faithful Kamino-class lending vault reference, PLANTED twin. Planted bug:
on `withdraw`, the on-chain `total_assets` bookkeeping field is debited
by `underlying + 1` even though only `underlying` lamports actually leave
the vault. Models the share-price-drift class of bug called out in the
bounty_hunter Kamino dossier ("collateral / share-accounting drift on
Critical tier"). Class: `balance_conservation`. Invariant:
`invariant_total_assets_conservation`.

## Summary

- Invariants total: **1**
- Invariants violated: **>=1**
- AI-suggested invariants in this run: **0**
- Runtime: **<200 ms** (planted bug fires on the first withdraw)
- Crucible version: `0.2.0`

This is the AUTHORED EXPECTATION. The unsuffixed `scorecard.md` sibling is
captured by `.github/workflows/ci.yml` from the real Crucible run on
every push; if that capture ever shows `0` violations on the planted
variant, the bug was not surfaced within the configured timeout and
the emit / planted-bug shape needs review.

---

cf-invariants-anchor — Apache-2.0. Operated by Michael Moffett — michael@caliperforge.com — team@caliperforge.com.
