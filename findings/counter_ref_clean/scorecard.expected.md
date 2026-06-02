# cf-invariants-anchor scorecard

## Summary

- Invariants total: **1**
- Invariants violated: **0**
- AI-suggested invariants in this run: **0**
- Invariant class: `monotonic_accounting`
- Crucible version: `0.2.0`

The clean counter_ref leaves `Vault.lifetime_deposited` alone in
`withdraw`. The monotonic invariant snapshots the field after every
action and asserts the next observation is `>=` snapshot; with the
ratchet intact, this holds across the full fuzz run.

---

cf-invariants-anchor — Apache-2.0. Operated by Michael Moffett — michael@caliperforge.com — team@caliperforge.com.
