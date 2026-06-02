# cf-invariants-anchor scorecard

## Summary

- Invariants total: **1**
- Invariants violated: **1**
- AI-suggested invariants in this run: **0**
- Invariant class: `access_control`
- Crucible version: `0.2.0`

## Counterexamples

### `invariant_withdraw_rejects_unauthorized`

- Minimal failing sequence (not shrunk; shrinking is v2):

```
INVARIANT VIOLATED invariant_withdraw_rejects_unauthorized
unauthorized withdraw succeeded on vault <PDA>
sequence: [action_attack_withdraw(N)]
```

The planted `Withdraw` accounts struct drops both the PDA seeds
constraint and `has_one = depositor`, so Anchor accepts the
attacker's Keypair signing as `depositor` over the existing vault
PDA. The withdraw body runs, lamports leave the vault and land in
the attacker's wallet — one action arm trips the sticky flag.

---

cf-invariants-anchor — Apache-2.0. Operated by Michael Moffett — michael@caliperforge.com — team@caliperforge.com.
