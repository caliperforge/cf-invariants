# cf-invariants-anchor scorecard

## Summary

- Invariants total: **1**
- Invariants violated: **1**
- AI-suggested invariants in this run: **0**
- Invariant class: `monotonic_accounting`
- Crucible version: `0.2.0`

## Counterexamples

### `invariant_lifetime_deposited_monotonic`

- Minimal failing sequence (not shrunk; shrinking is v2):

```
INVARIANT VIOLATED invariant_lifetime_deposited_monotonic
snapshot: 500
current:  0
sequence: [action_deposit(500), action_withdraw(500)]
```

The planted withdraw runs `lifetime_deposited = lifetime_deposited - amount`,
regressing the ratchet from 500 → 0 in one step. The fuzzer hits the
deposit-then-withdraw pair within the first few actions.

---

cf-invariants-anchor — Apache-2.0. Operated by Michael Moffett — michael@caliperforge.com — team@caliperforge.com.
