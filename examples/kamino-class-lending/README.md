# Kamino-class lending example

**Honest scope:** this is a **Kamino-*class* reference lending vault**, not the live Kamino
protocol. Kamino's production programs (`klend`, `kvault`) are **BUSL-1.1 licensed** and pinned
to **Anchor 0.29 / Solana 1.17**, so they are not cleanly harnessable with cf-invariants-anchor
(anchor-lang 1.0.1 / Solana 2.1.21). **No security finding is claimed against the live protocol.**

What this demonstrates: cf-invariants-anchor's lending-relevant invariant classes applied to a
representative lending vault — exercising the suggester (AI + heuristic) and emitting four
invariants:

- `total_assets_conservation` — vault assets are conserved across deposit/withdraw.
- `cumulative_yield_monotonic` — accrued yield only increases.
- `withdraw_rejects_unauthorized` — withdraw paths reject unauthorized callers.
- `set_admin_rejects_unauthorized` — admin transitions reject unauthorized callers.

Contents:
- `emitted/` — the four emitted invariant programs.
- `scorecards/` — suggester output (heuristic + AI path), captured during the run.

This example is a build-to-win proof point: it shows the tool's invariant authoring on a
lending surface. For a live open-source Solana protocol target (permissive license + current
toolchain), see the in-progress open-source-target work.

Built with AI assistance; AI-suggested invariants are tagged in source. Full policy at
caliperforge.com/ai-disclosure.
