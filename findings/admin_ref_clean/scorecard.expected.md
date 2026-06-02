# cf-invariants-anchor scorecard

## Summary

- Invariants total: **1**
- Invariants violated: **0**
- AI-suggested invariants in this run: **0**
- Invariant class: `access_control`
- Crucible version: `0.2.0`

The clean admin_ref's `Withdraw` enforces BOTH the PDA seeds
derivation (`seeds = [b"vault", depositor.key().as_ref()]`) AND
`has_one = depositor`. Every attacker probe — a freshly-minted
`Keypair` signing into the existing vault PDA — fails Anchor's
account validation before the program body executes. The fixture's
sticky `unauthorized_success_observed` flag never gets set, so the
invariant holds across the run.

---

cf-invariants-anchor — Apache-2.0. Operated by Michael Moffett — michael@caliperforge.com — team@caliperforge.com.
