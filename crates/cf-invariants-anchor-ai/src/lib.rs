// cf-invariants-anchor-ai — Anthropic-backed invariant suggester.
//
// This crate is the Anchor sibling of `cf-invariants-ai` (the Cairo
// path). It mirrors that crate's design contract one-for-one so an
// operator using both has a single mental model:
//
//   1. Every call carries a versioned prompt id (`PROMPT_VERSION`).
//      The prompt text lives in `prompts/invariant_suggestion_v1.txt`
//      and is loaded at runtime — a new version is a file add, not
//      an in-place edit.
//   2. Every returned candidate carries
//      `InvariantSource::AiSuggested { model, prompt_version, timestamp_utc }`.
//      The renderer's AI-disclosure banner is type-driven (see
//      `cf-invariants-anchor-report`), so the disclosure cannot be
//      silently dropped.
//   3. Every call writes a JSON audit-log entry to
//      `<audit_dir>/<timestamp>.json` (model, prompt version, token
//      counts, response SHA-256). `.cf-invariants-anchor/ai-log/` is
//      `.gitignore`d by default.
//   4. The HTTP transport is behind a trait so tests never touch
//      `reqwest`. The live transport (`LiveAnthropicTransport`) is
//      gated behind the `live` cargo feature; CI's workspace-tests
//      build never compiles it.
//   5. Per-call cost is computed in USD using verified Sonnet 4.6
//      pricing ($3/MTok input, $15/MTok output) and bounded at
//      $0.05/call by default (`PER_CALL_BUDGET_USD`). Over-budget
//      requests return `AiError::BudgetExceeded` before any state
//      is written.
//
// HARD-DISCLOSE: this module IS the AI in cf-invariants-anchor.
// Every invariant produced through `suggest_invariants` is marked
// `AiSuggested`; the scorecard renderer cannot omit the banner.

use cf_invariants_anchor_core::{
    ContractSurface, EmitHints, InvariantCandidate, InvariantSource,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Pinned model id. Sonnet 4.6 is the default tier for invariant
/// suggestion (scoped structural reasoning over typed input).
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-6";

/// Prompt version. Bumped (`_v2`, `_v3`, ...) when the prompt text
/// changes — never silently rewritten.
pub const PROMPT_VERSION: &str = "invariant_suggestion_v1";

/// Anthropic Sonnet 4.6 pricing per MTok, used for cost computation +
/// the per-call budget guard.
pub const SONNET_4_6_INPUT_USD_PER_MTOK: f64 = 3.0;
pub const SONNET_4_6_OUTPUT_USD_PER_MTOK: f64 = 15.0;

/// Per-call USD cap. Mirrors the cf-invariants Cairo budget.
pub const PER_CALL_BUDGET_USD: f64 = 0.05;

/// Strict-JSON candidate shape Claude returns. The mapper attaches
/// `InvariantSource::AiSuggested` on the way out — the model is not
/// trusted to self-disclose.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RawCandidate {
    pub name: String,
    pub summary: String,
    pub class: String,
    pub rank: f32,
    pub rationale: String,
    pub emit_hints: EmitHints,
}

#[derive(Clone, Debug)]
pub struct SuggestRequest<'a> {
    pub surface: &'a ContractSurface,
    /// Optional caller-side hint (e.g. "vault-style", "amm-style").
    /// Threaded into the prompt template; ignored by `MockTransport`.
    pub hint: Option<&'a str>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SuggestResponse {
    pub candidates: Vec<InvariantCandidate>,
    pub model: String,
    pub prompt_version: String,
    pub timestamp_utc: String,
    pub tokens_input: u32,
    pub tokens_output: u32,
    pub cost_usd: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub timestamp_utc: String,
    pub model: String,
    pub prompt_version: String,
    pub program_name: String,
    pub tokens_input: u32,
    pub tokens_output: u32,
    pub cost_usd: f64,
    /// SHA-256 hex of the raw response text. Leakage of a single
    /// suggestion can be reconstructed without re-storing the body.
    pub response_hash: String,
    pub candidates_returned: usize,
}

#[derive(thiserror::Error, Debug)]
pub enum AiError {
    #[error("anthropic api error: {0}")]
    Api(String),
    #[error("response was not valid invariant JSON: {0}")]
    BadResponse(String),
    #[error("prompt file not found or unreadable: {0}")]
    PromptMissing(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("budget exceeded: this call would cost ${cost:.4} (cap ${cap:.4})")]
    BudgetExceeded { cost: f64, cap: f64 },
}

/// Transport trait so unit tests never touch the wire. The live
/// implementation is `LiveAnthropicTransport` (feature-gated); tests
/// use `MockTransport`.
#[async_trait::async_trait]
pub trait AnthropicTransport: Send + Sync {
    /// Send a prompt; return `(raw_response_body, tokens_input, tokens_output)`.
    /// The body MUST be the strict-JSON array of `RawCandidate` (the
    /// prompt instructs Claude to never wrap it in prose). The
    /// orchestrator tolerates one level of `` ```json ... ``` `` fence.
    async fn send(
        &self,
        model: &str,
        prompt: &str,
    ) -> Result<(String, u32, u32), AiError>;
}

/// Mock transport for unit tests. Returns a canned body + canned
/// token counts; no network or env access.
pub struct MockTransport {
    pub canned_body: String,
    pub tokens_in: u32,
    pub tokens_out: u32,
}

#[async_trait::async_trait]
impl AnthropicTransport for MockTransport {
    async fn send(
        &self,
        _model: &str,
        _prompt: &str,
    ) -> Result<(String, u32, u32), AiError> {
        Ok((self.canned_body.clone(), self.tokens_in, self.tokens_out))
    }
}

/// Live Anthropic Messages-API transport. Only compiled with the
/// `live` cargo feature so CI's hermetic workspace-tests never
/// transitively pulls reqwest / tokio runtime.
#[cfg(feature = "live")]
pub struct LiveAnthropicTransport {
    pub api_key: String,
    pub endpoint: String,
}

#[cfg(feature = "live")]
impl LiveAnthropicTransport {
    /// Construct from env. Requires `ANTHROPIC_API_KEY` AND
    /// `CF_INVARIANTS_ANCHOR_AI_LIVE=1` — the second gate exists so
    /// a stray API key on the developer's machine doesn't accidentally
    /// fire a live call during `cargo run`.
    pub fn from_env() -> Result<Self, AiError> {
        if std::env::var("CF_INVARIANTS_ANCHOR_AI_LIVE")
            .ok()
            .as_deref()
            != Some("1")
        {
            return Err(AiError::Api(
                "CF_INVARIANTS_ANCHOR_AI_LIVE=1 not set — live AI path disabled".into(),
            ));
        }
        let key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| AiError::Api("ANTHROPIC_API_KEY not set".into()))?;
        Ok(Self {
            api_key: key,
            endpoint: "https://api.anthropic.com/v1/messages".to_string(),
        })
    }
}

#[cfg(feature = "live")]
#[async_trait::async_trait]
impl AnthropicTransport for LiveAnthropicTransport {
    async fn send(
        &self,
        model: &str,
        prompt: &str,
    ) -> Result<(String, u32, u32), AiError> {
        let body = serde_json::json!({
            "model": model,
            "max_tokens": 4096,
            "messages": [{"role": "user", "content": prompt}],
        });
        let client = reqwest::Client::new();
        let resp = client
            .post(&self.endpoint)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::Api(e.to_string()))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| AiError::Api(e.to_string()))?;
        if !status.is_success() {
            return Err(AiError::Api(format!("status {status}: {text}")));
        }
        let parsed: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| AiError::BadResponse(format!("envelope: {e}")))?;
        // Anthropic Messages returns `{ content: [{type:"text", text: "..."}],
        // usage: {input_tokens, output_tokens} }`. We pull the first text
        // block as the raw body and the usage block for counts.
        let raw = parsed
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| AiError::BadResponse("no content[0].text in response".into()))?
            .to_string();
        let usage = parsed.get("usage").cloned().unwrap_or_default();
        let tin = usage
            .get("input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let tout = usage
            .get("output_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        Ok((raw, tin, tout))
    }
}

/// AnthropicClient orchestrates the AI suggestion flow end-to-end.
///
/// Lifecycle: render prompt → transport call → cost guard → strict-JSON
/// parse → map onto `InvariantCandidate` with `AiSuggested` provenance
/// attached → audit-log write → return.
pub struct AnthropicClient<T: AnthropicTransport> {
    pub transport: T,
    pub model: String,
    pub audit_log_dir: PathBuf,
    pub prompt_path: PathBuf,
    pub per_call_budget_usd: f64,
}

impl<T: AnthropicTransport> AnthropicClient<T> {
    pub fn new(transport: T, audit_log_dir: PathBuf, prompt_path: PathBuf) -> Self {
        Self {
            transport,
            model: DEFAULT_MODEL.to_string(),
            audit_log_dir,
            prompt_path,
            per_call_budget_usd: PER_CALL_BUDGET_USD,
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_budget_usd(mut self, cap: f64) -> Self {
        self.per_call_budget_usd = cap;
        self
    }

    /// HARD-DISCLOSE: every candidate returned by this method carries
    /// `InvariantSource::AiSuggested { model, prompt_version, timestamp_utc }`.
    /// The renderer in `cf-invariants-anchor-report` keys the AI banner
    /// on `Scorecard.ai_suggestions_included > 0`, which is computed by
    /// counting candidates whose source `.is_ai_suggested()`. There is
    /// no in-band way to bypass disclosure.
    pub async fn suggest_invariants(
        &self,
        req: SuggestRequest<'_>,
    ) -> Result<SuggestResponse, AiError> {
        let prompt = self.render_prompt(&req)?;
        let (raw, tokens_in, tokens_out) =
            self.transport.send(&self.model, &prompt).await?;
        let cost = estimate_cost_usd(tokens_in, tokens_out);
        if cost > self.per_call_budget_usd {
            return Err(AiError::BudgetExceeded {
                cost,
                cap: self.per_call_budget_usd,
            });
        }
        let raw_candidates = parse_candidates(&raw)
            .map_err(|e| AiError::BadResponse(format!("{e}: {raw}")))?;
        let timestamp_utc = current_utc_iso();
        let source = InvariantSource::AiSuggested {
            model: self.model.clone(),
            prompt_version: PROMPT_VERSION.to_string(),
            timestamp_utc: timestamp_utc.clone(),
        };
        let candidates: Vec<InvariantCandidate> = raw_candidates
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
        let response = SuggestResponse {
            candidates: candidates.clone(),
            model: self.model.clone(),
            prompt_version: PROMPT_VERSION.to_string(),
            timestamp_utc: timestamp_utc.clone(),
            tokens_input: tokens_in,
            tokens_output: tokens_out,
            cost_usd: cost,
        };
        let response_hash = sha256_hex(raw.as_bytes());
        write_audit_log(
            &self.audit_log_dir,
            &AuditLogEntry {
                timestamp_utc,
                model: self.model.clone(),
                prompt_version: PROMPT_VERSION.to_string(),
                program_name: req.surface.program_name.clone(),
                tokens_input: tokens_in,
                tokens_output: tokens_out,
                cost_usd: cost,
                response_hash,
                candidates_returned: candidates.len(),
            },
        )?;
        Ok(response)
    }

    fn render_prompt(&self, req: &SuggestRequest<'_>) -> Result<String, AiError> {
        let template = std::fs::read_to_string(&self.prompt_path)
            .map_err(|_| AiError::PromptMissing(self.prompt_path.display().to_string()))?;
        let surface_json = serde_json::to_string_pretty(req.surface)
            .map_err(|e| AiError::BadResponse(e.to_string()))?;
        let hint = req.hint.unwrap_or("").to_string();
        Ok(template
            .replace("{{CONTRACT_SURFACE_JSON}}", &surface_json)
            .replace("{{AUTHOR_HINT_OR_EMPTY}}", &hint))
    }
}

/// Cost in USD given input/output tokens at the pinned Sonnet 4.6
/// pricing.
pub fn estimate_cost_usd(tokens_in: u32, tokens_out: u32) -> f64 {
    (tokens_in as f64) * SONNET_4_6_INPUT_USD_PER_MTOK / 1_000_000.0
        + (tokens_out as f64) * SONNET_4_6_OUTPUT_USD_PER_MTOK / 1_000_000.0
}

/// Parse the strict-JSON candidate array Claude returns. Tolerates
/// one level of `` ```json ... ``` `` code fence, rejects arbitrary
/// prose.
pub fn parse_candidates(raw: &str) -> Result<Vec<RawCandidate>, String> {
    let stripped = strip_code_fence(raw.trim());
    serde_json::from_str(stripped).map_err(|e| format!("strict-JSON parse failed: {e}"))
}

fn strip_code_fence(s: &str) -> &str {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("```json") {
        if let Some(inner) = rest.trim_start().strip_suffix("```") {
            return inner.trim();
        }
    }
    if let Some(rest) = s.strip_prefix("```") {
        if let Some(inner) = rest.trim_start().strip_suffix("```") {
            return inner.trim();
        }
    }
    s
}

/// Lint helper for tests: assert a candidate carries the AI-suggested
/// provenance. Used to catch a regression where disclosure could be
/// silently dropped.
pub fn assert_ai_disclosed(c: &InvariantCandidate) -> bool {
    matches!(c.source, InvariantSource::AiSuggested { .. })
}

/// Write an audit-log entry as JSON. Caller controls the directory;
/// `.cf-invariants-anchor/ai-log/` is the convention.
fn write_audit_log(dir: &Path, entry: &AuditLogEntry) -> Result<(), AiError> {
    std::fs::create_dir_all(dir)?;
    let stem = entry.timestamp_utc.replace(':', "-");
    let path = dir.join(format!("{stem}.json"));
    let json = serde_json::to_string_pretty(entry)
        .map_err(|e| AiError::BadResponse(e.to_string()))?;
    std::fs::write(path, json)?;
    Ok(())
}

/// UTC ISO-8601 timestamp (seconds precision). Avoids a `chrono` dep
/// for one timestamp use case; matches the cf-invariants Cairo shape.
fn current_utc_iso() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() as i64;
    let (y, mo, d, h, mi, se) = epoch_to_ymdhms(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{se:02}Z")
}

/// Howard-Hinnant Gregorian conversion (UTC).
fn epoch_to_ymdhms(secs: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let time_of_day = secs.rem_euclid(86_400) as u32;
    let h = time_of_day / 3600;
    let mi = (time_of_day / 60) % 60;
    let se = time_of_day % 60;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = (y + if m <= 2 { 1 } else { 0 }) as i32;
    (y, m, d, h, mi, se)
}

/// SHA-256 hex digest. In-crate to avoid a `sha2` dep for one
/// fingerprint use case.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let h = sha256(bytes);
    let mut s = String::with_capacity(64);
    for b in h.iter() {
        use std::fmt::Write;
        let _ = write!(&mut s, "{:02x}", b);
    }
    s
}

fn sha256(input: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1,
        0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
        0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
        0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
        0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h = [
        0x6a09e667u32, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c,
        0x1f83d9ab, 0x5be0cd19,
    ];
    let bit_len = (input.len() as u64) * 8;
    let mut data = input.to_vec();
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in data.chunks(64) {
        let mut w = [0u32; 64];
        for (i, c) in chunk.chunks(4).enumerate().take(16) {
            w[i] = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_invariants_anchor_core::{BalanceField, Instruction};

    fn vault_surface() -> ContractSurface {
        ContractSurface {
            program_id: "Va111tRef1111111111111111111111111111111111".into(),
            program_name: "vault_ref".into(),
            instructions: vec![
                Instruction { name: "initialize".into(), args: vec![],                accounts: vec![] },
                Instruction { name: "deposit".into(),    args: vec!["amount".into()], accounts: vec![] },
                Instruction { name: "withdraw".into(),   args: vec!["amount".into()], accounts: vec![] },
            ],
            balance_fields: vec![BalanceField {
                account: "Vault".into(),
                field: "amount".into(),
                ty: "u64".into(),
            }],
        }
    }

    fn canned_response_one_candidate() -> String {
        r#"[
          {
            "name": "invariant_amount_conservation",
            "summary": "Vault.amount == sum(deposits) - sum(withdrawals)",
            "class": "balance_conservation",
            "rank": 0.95,
            "rationale": "Standard conservation invariant for a single-depositor vault.",
            "emit_hints": {
              "account_type": "Vault",
              "field": "amount",
              "expected_expression": "fixture.expected_amount",
              "action_names": ["deposit", "withdraw"]
            }
          }
        ]"#.to_string()
    }

    fn prompt_path() -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.pop(); // crates/
        p.pop(); // workspace root
        p.push("prompts");
        p.push("invariant_suggestion_v1.txt");
        p
    }

    fn temp_audit_dir() -> PathBuf {
        let base = std::env::temp_dir();
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let p = base.join(format!("cfia-ai-{pid}-{nanos}"));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn cost_matches_pinned_pricing() {
        // 2500 input + 2000 output ≈ $0.0375 (under the 5-cent cap).
        let c = estimate_cost_usd(2500, 2000);
        assert!(c > 0.037 && c < 0.038, "got {c}");
    }

    #[test]
    fn parse_strict_json_succeeds() {
        let v = parse_candidates(&canned_response_one_candidate()).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].class, "balance_conservation");
    }

    #[test]
    fn parse_tolerates_code_fence() {
        let body = format!("```json\n{}\n```", canned_response_one_candidate());
        let v = parse_candidates(&body).unwrap();
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn parse_rejects_prose() {
        let body = "Here are the invariants: [{}]";
        assert!(parse_candidates(body).is_err());
    }

    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn timestamp_iso_utc_shape() {
        let s = current_utc_iso();
        assert_eq!(s.len(), 20, "expected 2026-MM-DDTHH:MM:SSZ, got {s}");
        assert!(s.ends_with('Z'));
        assert!(s.contains('T'));
    }

    #[tokio::test]
    async fn mock_roundtrip_attaches_ai_suggested_provenance() {
        let transport = MockTransport {
            canned_body: canned_response_one_candidate(),
            tokens_in: 2500,
            tokens_out: 2000,
        };
        let audit = temp_audit_dir();
        let client = AnthropicClient::new(transport, audit.clone(), prompt_path());
        let surface = vault_surface();
        let req = SuggestRequest { surface: &surface, hint: None };
        let resp = client.suggest_invariants(req).await.unwrap();
        assert_eq!(resp.model, DEFAULT_MODEL);
        assert_eq!(resp.prompt_version, PROMPT_VERSION);
        assert_eq!(resp.candidates.len(), 1);
        for c in &resp.candidates {
            // HARD-DISCLOSE contract: every returned candidate must
            // be tagged AiSuggested. If this ever flips, the scorecard
            // banner could silently drop.
            assert!(assert_ai_disclosed(c), "{c:?}");
            match &c.source {
                InvariantSource::AiSuggested {
                    model,
                    prompt_version,
                    timestamp_utc,
                } => {
                    assert_eq!(model, DEFAULT_MODEL);
                    assert_eq!(prompt_version, PROMPT_VERSION);
                    assert!(!timestamp_utc.is_empty());
                }
                _ => panic!("expected AiSuggested"),
            }
        }
        // Audit log entry written.
        let entries: Vec<_> = std::fs::read_dir(&audit).unwrap().collect();
        assert_eq!(entries.len(), 1);
        let _ = std::fs::remove_dir_all(&audit);
    }

    #[tokio::test]
    async fn mock_budget_guard_fires() {
        // 30k input + 20k output ≈ $0.39, well over the 5-cent cap.
        let transport = MockTransport {
            canned_body: canned_response_one_candidate(),
            tokens_in: 30_000,
            tokens_out: 20_000,
        };
        let audit = temp_audit_dir();
        let client = AnthropicClient::new(transport, audit.clone(), prompt_path());
        let surface = vault_surface();
        let req = SuggestRequest { surface: &surface, hint: None };
        let err = client.suggest_invariants(req).await.unwrap_err();
        match err {
            AiError::BudgetExceeded { cost, cap } => assert!(cost > cap),
            other => panic!("expected BudgetExceeded, got {other:?}"),
        }
        // No audit log entry on budget rejection.
        let entries: Vec<_> = std::fs::read_dir(&audit).unwrap().collect();
        assert!(entries.is_empty());
        let _ = std::fs::remove_dir_all(&audit);
    }

    #[tokio::test]
    async fn prompt_template_renders_with_surface_fields() {
        let transport = MockTransport {
            canned_body: canned_response_one_candidate(),
            tokens_in: 2500,
            tokens_out: 2000,
        };
        let audit = temp_audit_dir();
        let client = AnthropicClient::new(transport, audit.clone(), prompt_path());
        let surface = vault_surface();
        let req = SuggestRequest {
            surface: &surface,
            hint: Some("vault-style"),
        };
        let rendered = client.render_prompt(&req).unwrap();
        // Surface JSON appears interpolated, not the literal placeholder.
        assert!(rendered.contains("deposit"));
        assert!(rendered.contains("Vault"));
        assert!(!rendered.contains("{{CONTRACT_SURFACE_JSON}}"));
        let _ = std::fs::remove_dir_all(&audit);
    }

    #[tokio::test]
    async fn bad_response_returns_typed_error() {
        let transport = MockTransport {
            canned_body: "this is not JSON".to_string(),
            tokens_in: 1000,
            tokens_out: 1000,
        };
        let audit = temp_audit_dir();
        let client = AnthropicClient::new(transport, audit.clone(), prompt_path());
        let surface = vault_surface();
        let req = SuggestRequest { surface: &surface, hint: None };
        let err = client.suggest_invariants(req).await.unwrap_err();
        assert!(matches!(err, AiError::BadResponse(_)), "got {err:?}");
        let _ = std::fs::remove_dir_all(&audit);
    }
}
