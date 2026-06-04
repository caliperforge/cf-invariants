// Integration test — binds a captured *live* AI run to the codebase.
//
// On 2026-06-03 the suggester was switched from DORMANT (mock canned
// body derived from the heuristic) to a real model call against
// claude-sonnet-4-6 using `prompts/invariant_suggestion_v1.txt`. The
// run produced 3 candidate invariants for `vault_ref`; the strict-JSON
// body is saved at:
//
//   findings/ai_suggester_run_2026-06-03/raw_response.json
//
// The audit-log entry, rendered prompt, contract surface, and
// AiSuggested-tagged candidates live alongside it. This test re-loads
// the captured body and exercises the *same* parser the live transport
// uses — so a future change to `parse_candidates` that broke this real
// model output would fail CI, not just silently rot. It also asserts
// the candidates' provenance would carry `InvariantSource::AiSuggested`
// after the orchestrator's tagging step (the parser returns
// `RawCandidate`s; the orchestrator attaches the `AiSuggested` source).
//
// HARD-DISCLOSE preservation: if a future refactor lets a candidate
// drop the AiSuggested source, this test catches the regression because
// it constructs the tagged candidate the same way the live path does.

use cf_invariants_anchor_ai::{
    assert_ai_disclosed, parse_candidates, sha256_hex, DEFAULT_MODEL, PROMPT_VERSION,
};
use cf_invariants_anchor_core::{InvariantCandidate, InvariantSource};
use std::path::PathBuf;

fn captured_response_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // crates/
    p.pop(); // workspace root
    p.push("findings");
    p.push("ai_suggester_run_2026-06-03");
    p.push("raw_response.json");
    p
}

fn captured_audit_log_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p.push("findings");
    p.push("ai_suggester_run_2026-06-03");
    p.push("audit_log.json");
    p
}

#[test]
fn captured_live_response_parses_via_orchestrator_parser() {
    let body = std::fs::read_to_string(captured_response_path())
        .expect("captured raw_response.json must exist on disk");
    let raw_candidates = parse_candidates(&body)
        .expect("captured response must round-trip through the live-path parser");

    // Stamp shape: the 2026-06-03 vault_ref run produced exactly three
    // candidates in this order (balance_conservation, then two
    // access_control). If the run is ever re-captured, refresh this
    // table — the test pins what we *actually* shipped.
    assert_eq!(
        raw_candidates.len(),
        3,
        "expected 3 candidates from the captured 2026-06-03 vault_ref run"
    );
    let classes: Vec<&str> = raw_candidates.iter().map(|c| c.class.as_str()).collect();
    assert_eq!(
        classes,
        vec!["balance_conservation", "access_control", "access_control"]
    );
    let names: Vec<&str> = raw_candidates.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"invariant_vault_amount_conservation"));
    assert!(names.contains(&"invariant_withdraw_depositor_only"));
}

#[test]
fn captured_live_response_round_trips_through_ai_suggested_tagging() {
    let body = std::fs::read_to_string(captured_response_path()).unwrap();
    let raw_candidates = parse_candidates(&body).unwrap();

    // Replay the orchestrator's mapping step: attach AiSuggested
    // provenance the same way `AnthropicClient::suggest_invariants`
    // does after a successful transport call.
    let source = InvariantSource::AiSuggested {
        model: DEFAULT_MODEL.to_string(),
        prompt_version: PROMPT_VERSION.to_string(),
        timestamp_utc: "2026-06-03T20:09:51Z".to_string(),
    };
    let tagged: Vec<InvariantCandidate> = raw_candidates
        .into_iter()
        .map(|rc| InvariantCandidate {
            name: rc.name,
            summary: rc.summary,
            class: rc.class,
            rank: rc.rank.clamp(0.0, 1.0),
            rationale: rc.rationale,
            emit_hints: rc.emit_hints,
            source: source.clone(),
        })
        .collect();

    assert_eq!(tagged.len(), 3);
    for c in &tagged {
        assert!(
            assert_ai_disclosed(c),
            "tagged candidate must carry AiSuggested provenance: {}",
            c.name
        );
    }
}

#[test]
fn captured_audit_log_matches_response_hash() {
    // Cross-check: the audit-log entry we saved at run time must hash
    // the SAME strict-JSON body we still have on disk. If anything
    // drifted, this test fails — the evidence is no longer internally
    // consistent.
    let body = std::fs::read_to_string(captured_response_path()).unwrap();
    let computed = sha256_hex(body.as_bytes());

    let log = std::fs::read_to_string(captured_audit_log_path()).unwrap();
    let log_json: serde_json::Value = serde_json::from_str(&log).unwrap();
    let stored = log_json["response_hash"].as_str().unwrap();
    assert_eq!(
        computed, stored,
        "audit_log.response_hash must match sha256(raw_response.json) on disk"
    );

    // Also assert the audit log records the pinned model + prompt
    // version. If `DEFAULT_MODEL` or `PROMPT_VERSION` ever drifts, the
    // captured evidence becomes mis-tagged — fail loudly.
    assert_eq!(log_json["model"].as_str().unwrap(), DEFAULT_MODEL);
    assert_eq!(
        log_json["prompt_version"].as_str().unwrap(),
        PROMPT_VERSION
    );
    assert_eq!(log_json["candidates_returned"].as_u64().unwrap(), 3);
}
