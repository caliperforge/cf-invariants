# AI disclosure — cf-invariants-anchor

cf-invariants-anchor is built with AI assistance. This document is the
in-repo detail on what the AI module does, where the audit log lives,
and how to disable AI involvement.

## What is AI-suggested today

**Phase 1 AI suggester is live.** The default `cf-invariants-anchor suggest <idl>`
still runs the heuristic suggester (no AI call) — that path produces
`InvariantSource::Heuristic { suggester_version }` candidates and emits
no AI-disclosure banner. The new flag

```sh
cf-invariants-anchor suggest <idl> --ai
```

routes through the new `cf-invariants-anchor-ai` crate. Every candidate
returned through this path carries:

```
source: InvariantSource::AiSuggested {
    model: "claude-sonnet-4-6",
    prompt_version: "invariant_suggestion_v1",
    timestamp_utc: "<ISO-8601>",
}
```

and the scorecard renderer's AI-disclosure banner fires whenever any
such candidate is part of a run. The renderer's banner gate is type-
driven (`Scorecard.ai_suggestions_included > 0` is computed by counting
candidates whose source `.is_ai_suggested()`), so there is no in-band
way to bypass disclosure.

## Live vs. mock — what fires when

There are two transports behind the same `AnthropicTransport` trait:

- **`MockTransport`** — default. Returns a canned response derived
  deterministically from the heuristic suggester. Used when:
  - the CLI is built without `--features live-ai`, OR
  - `CF_INVARIANTS_ANCHOR_AI_LIVE` is unset / not equal to `1`, OR
  - `ANTHROPIC_API_KEY` is unset.

  This is the path CI runs — it exercises the full AI-provenance
  pipeline (parse → tag `AiSuggested` → write audit log) without
  needing an API key or network access, so the green badge is
  reproducible by anyone.

- **`LiveAnthropicTransport`** — actual Anthropic Messages API. Used
  when ALL of the following hold:
  - the CLI was built with `--features live-ai`,
  - `CF_INVARIANTS_ANCHOR_AI_LIVE=1` is set,
  - `ANTHROPIC_API_KEY=...` is set.

  The live path uses Claude Sonnet 4.6 with the pinned prompt at
  `prompts/invariant_suggestion_v1.txt`. Per-call cost is bounded at
  `$0.05` (the `PER_CALL_BUDGET_USD` constant); over-budget calls
  return `AiError::BudgetExceeded` before any state is written.

## Audit log

Every AI call (live OR mock) writes a JSON entry to
`.cf-invariants-anchor/ai-log/<timestamp>.json`. The entry records:

- timestamp (UTC ISO-8601, seconds),
- model id,
- prompt version,
- program name,
- token counts (input + output),
- cost in USD,
- SHA-256 hex digest of the raw response body (so a single suggestion
  can be re-reconstructed for review without re-storing the response),
- count of returned candidates.

The directory is `.gitignore`d by default. Override with
`cf-invariants-anchor suggest ... --audit-dir <path>`.

## AI assistance in **authoring** this codebase

This codebase was authored with AI assistance (Claude). Every file is
reviewed and accepted by the operator. The AI-authoring assistance is
distinct from the AI **runtime suggestion** path described above —
the former is one-time at authorship; the latter is a deliberate,
disclosed, audit-logged runtime feature.

## How to disable AI involvement entirely

- **Default build**: heuristic suggester only. No outbound calls, no
  API keys required. The `--ai` flag is rejected at config time if
  the binary was built without `--features live-ai` AND
  `CF_INVARIANTS_ANCHOR_AI_LIVE=1` — well, more precisely, with the
  live feature off the mock transport is used; users that want
  guaranteed-no-AI-tag output simply omit the `--ai` flag.
- **Live AI off, mock disclosure-tagged candidates off**: omit
  `--ai`. Default `suggest` returns `Heuristic`-tagged candidates
  only.

## Cost budget

The per-call cap is `PER_CALL_BUDGET_USD = $0.05`. The cf-invariants
(Cairo) sibling enforces a cumulative `WEEK2_DEMO_BUDGET_USD = $0.25`
across the three-contract demo; the Anchor sibling does not yet
ship a cross-call cumulative cap — it is queued for Phase 2 alongside
the second reference contract pair.

## Full policy

[caliperforge.com/ai-disclosure](https://caliperforge.com/ai-disclosure).
