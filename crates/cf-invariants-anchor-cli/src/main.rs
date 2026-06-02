// cf-invariants-anchor CLI.
//
// Surface:
//   cf-invariants-anchor version
//   cf-invariants-anchor ingest <idl.json> [--out path]
//   cf-invariants-anchor suggest <idl.json> [--ai] [--audit-dir path] [--out path]
//   cf-invariants-anchor emit <idl.json> [--target crucible|trident]
//                                        [--candidate-index N] [--out path]
//
// `suggest --ai` runs the full AI invariant-author flow against the
// pinned prompt (prompts/invariant_suggestion_v1.txt):
//   - Default build: uses `MockTransport` with a canned response derived
//     deterministically from the heuristic suggester's output. The
//     returned candidates ARE tagged `InvariantSource::AiSuggested` so
//     the scorecard AI-disclosure banner fires; an audit-log entry is
//     written to the configured `--audit-dir`. This keeps CI hermetic
//     (no API key, no network) while still exercising the full
//     provenance-tagging + audit-log path.
//   - With `--features live-ai` AND `CF_INVARIANTS_ANCHOR_AI_LIVE=1`
//     AND `ANTHROPIC_API_KEY` set: uses `LiveAnthropicTransport` and
//     actually calls Anthropic's Messages API.
//
// `run` (driving `crucible run`) is staged for Phase 2 — the Phase-0
// path leaves `crucible run` as the harness invocation.

use anyhow::{Context, Result};
use cf_invariants_anchor_ai::{
    AnthropicClient, AnthropicTransport, MockTransport, RawCandidate, SuggestRequest,
    DEFAULT_MODEL,
};
#[cfg_attr(not(test), allow(unused_imports))]
use cf_invariants_anchor_core::{ContractSurface, InvariantCandidate, InvariantSource};
use cf_invariants_anchor_emit::Target;
use cf_invariants_anchor_suggest::ClassRegistry;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "cf-invariants-anchor",
    version,
    about = "AI invariant-author for Crucible / Trident on Anchor programs."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Print cf-invariants-anchor version + target Crucible version.
    Version,

    /// Parse an Anchor IDL into a typed contract surface (JSON to stdout).
    Ingest {
        idl: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Propose ranked candidate invariants.
    ///
    /// Default: heuristic suggester (no AI call, candidates tagged
    /// `InvariantSource::Heuristic`).
    /// `--ai`: AI suggester path (candidates tagged
    /// `InvariantSource::AiSuggested { model, prompt_version, timestamp_utc }`,
    /// audit-log entry written). Uses MockTransport by default
    /// (deterministic, CI-safe); LiveAnthropicTransport when the CLI
    /// is built with `--features live-ai` AND
    /// `CF_INVARIANTS_ANCHOR_AI_LIVE=1` AND `ANTHROPIC_API_KEY` set.
    Suggest {
        idl: PathBuf,
        #[arg(long, default_value_t = false)]
        ai: bool,
        #[arg(long, default_value = ".cf-invariants-anchor/ai-log")]
        audit_dir: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Emit a Crucible-compatible (or Trident-stub) fuzz fixture file.
    Emit {
        idl: PathBuf,
        #[arg(long, default_value = "crucible")]
        target: String,
        #[arg(long, default_value_t = 0)]
        candidate_index: usize,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

const CRUCIBLE_TARGET_VERSION: &str = "0.2.0";

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Version => {
            println!(
                "cf-invariants-anchor {} (target: Crucible v{})",
                env!("CARGO_PKG_VERSION"),
                CRUCIBLE_TARGET_VERSION
            );
        }
        Command::Ingest { idl, out } => {
            let surface = cf_invariants_anchor_idl::ingest_path(&idl)?;
            let text = serde_json::to_string_pretty(&surface)?;
            write_out(out, &text)?;
        }
        Command::Suggest {
            idl,
            ai,
            audit_dir,
            out,
        } => {
            let surface = cf_invariants_anchor_idl::ingest_path(&idl)?;
            let candidates = if ai {
                suggest_via_ai(&surface, &audit_dir).await?
            } else {
                ClassRegistry::default().propose_all(&surface)
            };
            let text = serde_json::to_string_pretty(&candidates)?;
            write_out(out, &text)?;
        }
        Command::Emit {
            idl,
            target,
            candidate_index,
            out,
        } => {
            let surface = cf_invariants_anchor_idl::ingest_path(&idl)?;
            let candidates = ClassRegistry::default().propose_all(&surface);
            let candidate = candidates
                .get(candidate_index)
                .ok_or_else(|| anyhow::anyhow!("no candidate at index {candidate_index}"))?;
            let target = parse_target(&target)?;
            let rendered = cf_invariants_anchor_emit::render(&surface, candidate, target);
            write_out(out, &rendered)?;
        }
    }
    Ok(())
}

/// Wire the AI suggester path. Live transport is only available when
/// the CLI was built with `--features live-ai` AND the env gates are
/// set; otherwise we fall back to the MockTransport with a canned
/// response derived from the heuristic suggester — that path still
/// exercises the full AI provenance-tagging + audit-log machinery so
/// the disclosure round-trip is proven in CI.
async fn suggest_via_ai(
    surface: &ContractSurface,
    audit_dir: &PathBuf,
) -> Result<Vec<InvariantCandidate>> {
    let prompt_path = find_prompt_path()?;
    let live_requested = std::env::var("CF_INVARIANTS_ANCHOR_AI_LIVE").ok().as_deref()
        == Some("1");
    if live_requested {
        run_live_path(surface, audit_dir, prompt_path).await
    } else {
        run_mock_path(surface, audit_dir, prompt_path).await
    }
}

/// Mock path — exercises the AI module end-to-end without a real API
/// call. The canned body is built deterministically from the heuristic
/// suggester so the AI-tagged output reflects the same ranking.
async fn run_mock_path(
    surface: &ContractSurface,
    audit_dir: &PathBuf,
    prompt_path: PathBuf,
) -> Result<Vec<InvariantCandidate>> {
    let heuristic_candidates = ClassRegistry::default().propose_all(surface);
    let canned_body = canned_body_from_heuristic(&heuristic_candidates)?;
    let transport = MockTransport {
        canned_body,
        tokens_in: 2500,
        tokens_out: 2000,
    };
    run_with_transport(transport, surface, audit_dir, prompt_path).await
}

#[cfg(feature = "live-ai")]
async fn run_live_path(
    surface: &ContractSurface,
    audit_dir: &PathBuf,
    prompt_path: PathBuf,
) -> Result<Vec<InvariantCandidate>> {
    let transport = cf_invariants_anchor_ai::LiveAnthropicTransport::from_env()
        .context("CF_INVARIANTS_ANCHOR_AI_LIVE=1 set but live transport setup failed")?;
    run_with_transport(transport, surface, audit_dir, prompt_path).await
}

#[cfg(not(feature = "live-ai"))]
async fn run_live_path(
    _surface: &ContractSurface,
    _audit_dir: &PathBuf,
    _prompt_path: PathBuf,
) -> Result<Vec<InvariantCandidate>> {
    anyhow::bail!(
        "CF_INVARIANTS_ANCHOR_AI_LIVE=1 but this binary was not built with \
         `--features live-ai`. Either rebuild with the feature, or unset the \
         env var to fall back to the deterministic mock path."
    )
}

async fn run_with_transport<T: AnthropicTransport>(
    transport: T,
    surface: &ContractSurface,
    audit_dir: &PathBuf,
    prompt_path: PathBuf,
) -> Result<Vec<InvariantCandidate>> {
    let client = AnthropicClient::new(transport, audit_dir.clone(), prompt_path)
        .with_model(DEFAULT_MODEL);
    let req = SuggestRequest {
        surface,
        hint: None,
    };
    let resp = client
        .suggest_invariants(req)
        .await
        .context("AI suggester call failed")?;
    Ok(resp.candidates)
}

/// Serialize the heuristic candidates into the strict-JSON shape the
/// AI prompt asks Claude for. The mock transport returns this verbatim;
/// the `AnthropicClient` then parses it, re-attaches AiSuggested
/// provenance, and writes the audit-log entry.
fn canned_body_from_heuristic(
    heuristic: &[InvariantCandidate],
) -> Result<String> {
    let raw: Vec<RawCandidate> = heuristic
        .iter()
        .map(|c| RawCandidate {
            name: c.name.clone(),
            summary: c.summary.clone(),
            class: c.class.clone(),
            rank: c.rank,
            rationale: c.rationale.clone(),
            emit_hints: c.emit_hints.clone(),
        })
        .collect();
    Ok(serde_json::to_string(&raw)?)
}

fn find_prompt_path() -> Result<PathBuf> {
    // 1. CWD relative ("./prompts/invariant_suggestion_v1.txt").
    let cwd = std::env::current_dir()?;
    let cwd_p = cwd.join("prompts").join("invariant_suggestion_v1.txt");
    if cwd_p.exists() {
        return Ok(cwd_p);
    }
    // 2. Up one level (CI / harness running from references/<name>/).
    if let Some(parent) = cwd.parent() {
        let p = parent.join("prompts").join("invariant_suggestion_v1.txt");
        if p.exists() {
            return Ok(p);
        }
    }
    // 3. Up two levels.
    if let Some(gp) = cwd.parent().and_then(|p| p.parent()) {
        let p = gp.join("prompts").join("invariant_suggestion_v1.txt");
        if p.exists() {
            return Ok(p);
        }
    }
    // 4. Env override.
    if let Ok(env_path) = std::env::var("CF_INVARIANTS_ANCHOR_PROMPT_PATH") {
        let p = PathBuf::from(env_path);
        if p.exists() {
            return Ok(p);
        }
    }
    anyhow::bail!(
        "could not locate prompts/invariant_suggestion_v1.txt — checked CWD, parent, \
         grandparent; set CF_INVARIANTS_ANCHOR_PROMPT_PATH to override"
    )
}

fn parse_target(s: &str) -> Result<Target> {
    match s.to_ascii_lowercase().as_str() {
        "crucible" => Ok(Target::Crucible),
        "trident" => Ok(Target::Trident),
        other => anyhow::bail!("unknown target `{other}` (expected `crucible` | `trident`)"),
    }
}

fn write_out(out: Option<PathBuf>, text: &str) -> Result<()> {
    match out {
        Some(path) => {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            std::fs::write(path, text)?;
        }
        None => {
            print!("{text}");
            if !text.ends_with('\n') {
                println!();
            }
        }
    }
    Ok(())
}

// `--ai` smoke test: heuristic-derived candidates round-tripped through
// the AI module produce `AiSuggested`-tagged candidates with an audit
// log entry. Lives in main.rs because the canned-body helper is here.
#[cfg(test)]
mod tests {
    use super::*;
    use cf_invariants_anchor_core::{BalanceField, Instruction};

    fn vault_surface() -> ContractSurface {
        ContractSurface {
            program_id: "Va111tRef1111111111111111111111111111111111".into(),
            program_name: "vault_ref".into(),
            instructions: vec![
                Instruction {
                    name: "deposit".into(),
                    args: vec!["amount".into()],
                    accounts: vec!["vault".into(), "depositor".into()],
                },
                Instruction {
                    name: "withdraw".into(),
                    args: vec!["amount".into()],
                    accounts: vec!["vault".into(), "depositor".into()],
                },
            ],
            balance_fields: vec![BalanceField {
                account: "Vault".into(),
                field: "amount".into(),
                ty: "u64".into(),
            }],
        }
    }

    fn temp_dir() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "cfia-cli-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[tokio::test]
    async fn ai_path_via_mock_attaches_ai_suggested_to_every_candidate() {
        let surface = vault_surface();
        let audit = temp_dir();
        let candidates = run_mock_path(&surface, &audit, find_test_prompt()).await.unwrap();
        assert!(!candidates.is_empty());
        for c in &candidates {
            assert!(
                matches!(c.source, InvariantSource::AiSuggested { .. }),
                "candidate {} not AI-tagged: {:?}",
                c.name,
                c.source
            );
        }
        // Audit log entry was written.
        let entries: Vec<_> = std::fs::read_dir(&audit).unwrap().collect();
        assert_eq!(entries.len(), 1);
        let _ = std::fs::remove_dir_all(&audit);
    }

    fn find_test_prompt() -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.pop(); // crates/
        p.pop(); // workspace root
        p.push("prompts");
        p.push("invariant_suggestion_v1.txt");
        p
    }
}
