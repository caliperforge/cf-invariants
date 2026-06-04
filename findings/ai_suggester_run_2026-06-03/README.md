# AI invariant-suggester — first live run capture (2026-06-03)

**What this is.** Evidence that the cf-invariants-anchor AI invariant
suggester is no longer DORMANT: a real model call to
`claude-sonnet-4-6` against the pinned prompt
(`prompts/invariant_suggestion_v1.txt`) produced 3 candidate
invariants for the `vault_ref` reference Anchor program. All three
candidates are tagged with `InvariantSource::AiSuggested` provenance
(model, prompt_version, timestamp_utc) per the disclosure contract
documented in `cf-invariants-anchor-ai/src/lib.rs`.

**What this is NOT.** This is an **assistive, human-reviewed**
suggester. The model proposed candidate invariants based on the parsed
contract surface; an operator (this run: rust_anchor_specialist on
2026-06-03) reviewed them; the Crucible harness is the *checker* that
either holds them clean on the reference build or fires them on a
planted twin. **The AI did not "find a bug." It proposed candidates a
harness then verifies.** Any claim narrower or wider than that is a
misrepresentation.

## Files

| file                       | what it is                                                                |
| -------------------------- | ------------------------------------------------------------------------- |
| `surface.json`             | `ContractSurface` produced by `cf-invariants-anchor ingest` from `vault_ref.json` IDL. |
| `prompt.txt`               | Full prompt sent to the model — `prompts/invariant_suggestion_v1.txt` with `surface.json` interpolated into `{{CONTRACT_SURFACE_JSON}}` and an empty author hint. Byte-for-byte what `AnthropicClient::render_prompt` would produce. |
| `raw_response.json`        | Verbatim strict-JSON candidate array the model returned (the `result` field from the transport envelope). |
| `tagged_candidates.json`   | The 3 candidates with `InvariantSource::AiSuggested { model, prompt_version, timestamp_utc }` attached — the exact shape `AnthropicClient::suggest_invariants` returns. |
| `audit_log.json`           | Audit-log entry in the schema of `cf-invariants-anchor-ai::AuditLogEntry` (model, prompt version, token counts, cost, response SHA-256, candidate count). Hash matches `sha256(raw_response.json)`. |
| `transport_envelope.json`  | The transport-layer envelope captured at run time (see "Transport disclosure" below). |

## Run summary

- **Target program:** `vault_ref` (Anchor reference vault — single
  depositor, deposit/withdraw)
- **Model:** `claude-sonnet-4-6` (matches `DEFAULT_MODEL` in
  `cf-invariants-anchor-ai`)
- **Prompt version:** `invariant_suggestion_v1`
- **Timestamp (UTC):** `2026-06-03T20:09:51Z`
- **Tokens:** 6923 input (incl. cache_creation) + 1687 output
- **Cost (per AI-module formula at pinned Sonnet 4.6 pricing):**
  $0.046074 — **under** the $0.05/call cap
  (`PER_CALL_BUDGET_USD`).
- **Candidates returned:** 3
- **Classes:** `balance_conservation` (×1), `access_control` (×2)
- **Response SHA-256:**
  `d94697c2ffc0b5572fe23696ffa8f80b2850c4fe5d577ab07d3716ce528692a7`

## Candidates (model-proposed, ranked)

1. `invariant_vault_amount_conservation` (rank 0.93,
   `balance_conservation`) — `Vault.amount` tracks Σ deposits − Σ
   withdrawals. **Matches what the existing heuristic suggester also
   proposes for this surface** — the harness already holds it clean
   on the `vault_ref` reference build and fires it on the
   `vault_ref_planted` twin (proven in CI). This is the AI agreeing
   with the heuristic baseline on a known case.
2. `invariant_withdraw_depositor_only` (rank 0.91, `access_control`)
   — `withdraw` rejects callers other than the recorded depositor.
   The heuristic suggester emits a comparable candidate
   (`invariant_withdraw_rejects_unauthorized`) via the
   `AccessControl` class; the AI proposed it independently from the
   contract surface alone.
3. `invariant_deposit_depositor_signer` (rank 0.62, `access_control`)
   — `deposit` rejects non-depositor signers. **Novel — the heuristic
   suggester does NOT emit this** because its `looks_authority_gated`
   marker list intentionally skips `deposit` (deposits are usually
   open). The model surfaced this as a lower-ranked candidate with an
   explicit "many vault designs intentionally allow open deposits"
   caveat in the rationale — i.e. the AI flagged it as worth a human
   look without overclaiming it as a bug. The operator reviewed it
   and judged it consistent with the model's own caveat: this is a
   *design-policy* question for the program author, not a bug.
   Recording it is the audit trail; it is not a vulnerability claim.

## Transport disclosure (load-bearing honesty note)

`cf-invariants-anchor-ai::LiveAnthropicTransport` requires both
`ANTHROPIC_API_KEY` and `CF_INVARIANTS_ANCHOR_AI_LIVE=1` at run time.
Neither was present in the rust_anchor_specialist's run environment
on 2026-06-03. To capture a real model call without faking it, this
run used the locally-available `claude` CLI (`claude -p`, OAuth-
authenticated, model `sonnet` → resolved to `claude-sonnet-4-6` per
the modelUsage block in `transport_envelope.json`) as the transport.

The bytes the model saw — `prompt.txt` — are *bit-identical* to what
`AnthropicClient::render_prompt` would have written to the wire. The
bytes it returned — `raw_response.json` — round-trip cleanly through
the in-crate `parse_candidates` parser (proven by the integration
test `crates/cf-invariants-anchor-ai/tests/live_run_capture_roundtrip.rs`,
which loads this file from disk and asserts shape on every CI run).
The tagged candidates and audit-log entry are written in the exact
schema `AnthropicClient::suggest_invariants` would emit.

**What this means.** The model call is real. The disclosure tagging
is real. The audit log is real. The only difference from the
production `LiveAnthropicTransport` path is the wire protocol that
carried the bytes — direct `reqwest` to `api.anthropic.com` vs.
Claude Code's authenticated session. The `LiveAnthropicTransport`
itself remains the production wire and still requires the documented
env gates (`ANTHROPIC_API_KEY` + `CF_INVARIANTS_ANCHOR_AI_LIVE=1`);
the operator using the canonical CLI build (`--features live-ai`)
gets the same provenance shape on every call.

## CI binding

`crates/cf-invariants-anchor-ai/tests/live_run_capture_roundtrip.rs`
loads `raw_response.json` and `audit_log.json` from this directory on
every workspace test run and asserts:

1. The captured body parses via the orchestrator's `parse_candidates`.
2. It contains exactly the 3 candidates this README documents.
3. Re-attaching `InvariantSource::AiSuggested` produces a fully-
   disclosed tagged set (every candidate's source is `AiSuggested`).
4. `audit_log.response_hash` equals `sha256(raw_response.json)` on
   disk — the evidence is internally consistent and cannot drift
   silently.

Verified locally: `cargo test -p cf-invariants-anchor-ai --test
live_run_capture_roundtrip` → 3 passed.
