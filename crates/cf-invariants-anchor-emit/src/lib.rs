// cf-invariants-anchor-emit — IDL-driven Crucible fixture codegen.
//
// R1 rewrite (T-P031-R1): the previous emitter was a fill-in-the-blanks
// copy of the `vault_ref` example — it hardcoded that program's
// instruction names, argument shapes, account structs, and PDA seeds
// for EVERY target, and failed compilation with 19 errors on the first
// real program (Raydium CP-Swap). This version renders everything from
// the parsed Anchor 0.30+ IDL:
//
//   - Standalone client bindings are GENERATED into the fixture:
//     instruction data structs (IDL discriminators + manual
//     `AnchorSerialize`/`InstructionData` impls), account-meta structs
//     (manual `ToAccountMetas` from IDL writable/signer flags), and
//     state-account structs (manual `AnchorDeserialize` +
//     `Discriminator` from the IDL type layouts). The target crate is
//     NOT a dependency of the harness, so the target's anchor-lang
//     version, module layout, and crate name no longer matter
//     (C1 friction F3/F4/F7).
//   - `setup()` derives PDA addresses from the IDL seed specs, creates
//     one funded Keypair per IDL signer account, and parses IDL-pinned
//     fixed addresses (F5).
//   - Action arms are rendered per IDL instruction with that
//     instruction's real argument names/types and account list (F6).
//
// Where the IDL genuinely does not carry data the fixture would need,
// rendering FAILS with `EmitError::MissingIdlData` naming the exact
// gap — no heuristic fallbacks, no reference-example defaults (the
// R1 kill-condition discipline). Instructions that cannot be rendered
// faithfully (e.g. a signer pinned to a fixed address whose key we
// cannot hold) are EXCLUDED with a per-instruction reason emitted into
// the fixture header, not silently faked.
//
// Class dispatch (unchanged):
//   - balance_conservation  -> ledger fixture + fuzz_assert_eq
//   - monotonic_accounting  -> snapshot fixture + fuzz_assert_le
//   - access_control        -> attacker probe + sticky flag
//
// Crucible API note: the emit target remains the real v0.2.0 surface —
// `#[fuzz_fixture]` + `#[invariant_test]` from `crucible_fuzzer`, with
// `fuzz_assert_*` assertion macros. Trident stays a stub.

use cf_invariants_anchor_core::{
    ContractSurface, FieldDef, IdlType, Instruction, InvariantCandidate, InvariantSource, PdaDef,
    RelationSpec, SeedDef, TypeDef, TypeDefBody,
};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// Which downstream harness we render for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Asymmetric Research's Crucible (LibAFL + LiteSVM). Default.
    Crucible,
    /// Ackee's Trident (TridentSVM, manual invariants). Phase 1.
    /// Stubbed — emit returns an explanatory placeholder so the
    /// `cli emit --target trident …` path is reachable.
    Trident,
}

/// Rendering failed because the IDL does not carry required data, or
/// because the candidate cannot be encoded faithfully. These are LOUD
/// by design: the alternative (a hardcoded template) is the defect the
/// R1 rewrite removed.
#[derive(Debug, Error)]
pub enum EmitError {
    #[error("the Anchor IDL does not carry data this fixture needs: {0}")]
    MissingIdlData(String),
    #[error("unsupported by the emitter: {0}")]
    Unsupported(String),
    #[error(
        "class `{0}` not recognized by cf-invariants-anchor-emit. Either teach the emitter \
         how to render this class, or remove the candidate from the suggester path. \
         See docs/architecture.md §emit-classes."
    )]
    UnknownClass(String),
}

/// Knobs the CLI exposes.
#[derive(Debug, Clone, Default)]
pub struct EmitOptions {
    /// Path (relative to the harness crate) of the target program's
    /// compiled `.so`. Default: `../../target/deploy/<name>.so` — the
    /// reference-pair layout. For a deployed mainnet target, dump the
    /// binary with `solana program dump <PROGRAM_ID> <name>.so` and
    /// point this at it.
    pub program_so: Option<String>,
    /// Path (relative to the harness crate) of a directory containing
    /// mainnet account snapshots (`snapshot_<pubkey>.json` per account,
    /// as produced by `solana account <pk> --output json --output-file
    /// snapshot_<pk>.json`, plus an optional `manifest.json` naming the
    /// alias bindings). When present the fixture preloads these into
    /// the SVM before the fuzz loop so instructions execute against
    /// real, non-trivial state instead of a blank ledger.
    pub snapshot_dir: Option<String>,
}

/// Render the candidate as a fuzz-test source string.
pub fn render(
    surface: &ContractSurface,
    candidate: &InvariantCandidate,
    target: Target,
) -> Result<String, EmitError> {
    render_with_options(surface, candidate, target, &EmitOptions::default())
}

pub fn render_with_options(
    surface: &ContractSurface,
    candidate: &InvariantCandidate,
    target: Target,
    opts: &EmitOptions,
) -> Result<String, EmitError> {
    match target {
        Target::Crucible => render_crucible(surface, candidate, opts),
        Target::Trident => Ok(render_trident_stub(surface, candidate)),
    }
}

fn render_crucible(
    surface: &ContractSurface,
    candidate: &InvariantCandidate,
    opts: &EmitOptions,
) -> Result<String, EmitError> {
    let gen = Generator::build(surface, candidate, opts)?;
    match candidate.class.as_str() {
        "balance_conservation" => gen.render_balance(),
        "monotonic_accounting" => gen.render_monotonic(),
        "access_control" => gen.render_access_control(),
        "relation_invariants" => gen.render_relation_bundle(),
        other => Err(EmitError::UnknownClass(other.to_string())),
    }
}

// ---------------------------------------------------------------------------
// Generation context.
// ---------------------------------------------------------------------------

/// How a named account resolves to an address, program-wide.
#[derive(Debug, Clone)]
enum Role {
    /// IDL pins a base58 address.
    Fixed(String),
    /// IDL marks it a signer somewhere — the fixture holds a funded
    /// Keypair for it.
    Signer,
    /// Some instruction defines a PDA whose seeds resolve without
    /// per-call arguments — derivable once in `setup()`.
    StablePda(PdaDef),
    /// The IDL constrains nothing about this account. The fixture
    /// allocates one stable placeholder Pubkey per name so every
    /// instruction that references the name targets the same address.
    Placeholder,
}

struct Generator<'a> {
    surface: &'a ContractSurface,
    candidate: &'a InvariantCandidate,
    program_so: String,
    snapshot_dir: Option<String>,
    /// Instructions renderable as fuzz action arms.
    supported: Vec<&'a Instruction>,
    /// name -> reason, for the header comment.
    excluded: Vec<(String, String)>,
    /// Account-name -> resolution role (across supported instructions).
    roles: BTreeMap<String, Role>,
}

impl<'a> Generator<'a> {
    fn build(
        surface: &'a ContractSurface,
        candidate: &'a InvariantCandidate,
        opts: &EmitOptions,
    ) -> Result<Self, EmitError> {
        let program_so = opts
            .program_so
            .clone()
            .unwrap_or_else(|| format!("../../target/deploy/{}.so", surface.program_name));

        let mut supported = Vec::new();
        let mut excluded = Vec::new();
        for ix in &surface.instructions {
            match instruction_supported(ix) {
                Ok(()) => supported.push(ix),
                Err(reason) => excluded.push((ix.name.clone(), reason)),
            }
        }
        if supported.is_empty() {
            return Err(EmitError::Unsupported(format!(
                "no instruction in this IDL is renderable as a fuzz action: {}",
                excluded
                    .iter()
                    .map(|(n, r)| format!("`{n}`: {r}"))
                    .collect::<Vec<_>>()
                    .join("; ")
            )));
        }

        let roles = assign_roles(&supported);

        Ok(Self {
            surface,
            candidate,
            program_so,
            snapshot_dir: opts.snapshot_dir.clone(),
            supported,
            excluded,
            roles,
        })
    }

    // -- shared rendering pieces --------------------------------------------

    fn fixture_name(&self, suffix: &str) -> String {
        format!(
            "{}{}Fixture",
            capitalize(&self.surface.program_name.replace(['_', '-'], "")),
            suffix
        )
    }

    fn header(&self, fixture_doc: &str) -> String {
        let invariant_fn = &self.candidate.name;
        let class = &self.candidate.class;
        let disclosure = disclosure_header(&self.candidate.source);
        let summary = &self.candidate.summary;
        let mut excluded_block = String::new();
        if !self.excluded.is_empty() {
            excluded_block.push_str(
                "//\n// Instructions NOT rendered as fuzz actions (per-instruction reason —\n\
                 // the IDL does not carry what a faithful encoding would need):\n",
            );
            for (name, reason) in &self.excluded {
                excluded_block.push_str(&format!("//   - `{name}`: {reason}\n"));
            }
        }
        format!(
            "// {invariant_fn}\n\
             //\n\
             // Emitted by cf-invariants-anchor (IDL-driven codegen) for the {class} class.\n\
             // Target: Crucible v0.2.0 (asymmetric-research/crucible).\n\
             // {disclosure}\n\
             //\n\
             // {summary}\n\
             //\n\
             {fixture_doc}\n\
             //\n\
             // Every instruction shape below — names, discriminators, argument\n\
             // types, account lists, PDA seeds, fixed addresses — is rendered\n\
             // from this program's Anchor IDL. The target crate is NOT a\n\
             // dependency of this harness: the bindings are standalone, so the\n\
             // target's anchor-lang version and module layout do not constrain\n\
             // the build.\n\
             {excluded_block}"
        )
    }

    fn imports_and_consts(&self) -> String {
        let snapshot_const = match &self.snapshot_dir {
            Some(dir) => format!(
                "\n/// Mainnet account snapshot directory (relative to harness). The\n\
                 /// fixture preloads every `snapshot_<pubkey>.json` here into the\n\
                 /// SVM before the fuzz loop, so instructions execute against real,\n\
                 /// non-trivial state instead of a blank ledger. If the directory\n\
                 /// does not exist at run time, preload is skipped silently and the\n\
                 /// fixture runs against a blank SVM (the R1 behavior).\n\
                 const SNAPSHOT_DIR: &str = \"{dir}\";\n\
                 /// Alias manifest inside SNAPSHOT_DIR: maps IDL-account names\n\
                 /// (e.g. `pool_state`, `lp_mint`, `token_0_vault`) to the\n\
                 /// mainnet base58 pubkey the snapshot file is keyed on. The\n\
                 /// fixture uses this to rebind its `a_<name>` locals AFTER\n\
                 /// creating its own placeholders, so subsequent action arms\n\
                 /// address the ingested state.\n\
                 const SNAPSHOT_MANIFEST: &str = \"manifest.json\";"
            ),
            None => String::new(),
        };
        format!(
            "#![allow(unused_imports, dead_code, unused_variables, clippy::too_many_arguments)]\n\
             \n\
             use crucible_fuzzer::anchor_lang::solana_program::instruction::AccountMeta;\n\
             use crucible_fuzzer::anchor_lang::system_program;\n\
             use crucible_fuzzer::anchor_lang::{{self}};\n\
             use crucible_fuzzer::*;\n\
             use solana_keypair::Keypair;\n\
             use solana_pubkey::Pubkey;\n\
             use solana_signer::Signer;\n\
             use std::rc::Rc;\n\
             use std::str::FromStr;\n\
             \n\
             const INITIAL_BALANCE: u64 = 10_000_000_000;\n\
             /// Program id, from the IDL `address` field.\n\
             const PROGRAM_ID: &str = \"{}\";\n\
             /// Compiled target program. For a deployed program, produce this\n\
             /// with: `solana program dump {} <name>.so`.\n\
             const PROGRAM_SO: &str = \"{}\";{snapshot_const}",
            self.surface.program_id, self.surface.program_id, self.program_so
        )
    }

    /// Snapshot-preload block for `setup()`. Reads a manifest JSON of
    /// `{"aliases": {"<idl_name>": "<mainnet_pubkey_base58>"}}` and,
    /// for each entry, loads `snapshot_<pubkey>.json` (as produced by
    /// `solana account ... --output json`) into the SVM via
    /// `ctx.create_account().data(...).owner(...).lamports(...).create()`
    /// and REBINDS the local `a_<idl_name>` to the mainnet pubkey so
    /// action arms address the ingested state.
    ///
    /// Silently no-ops if `SNAPSHOT_DIR` doesn't exist at run time.
    /// Emit-time: contributes to `setup()`; empty string when no
    /// snapshot dir was configured.
    fn snapshot_preload(&self, aliasable: &[String]) -> String {
        if self.snapshot_dir.is_none() {
            return String::new();
        }
        // Only rebind locals we actually produced (placeholders + PDAs).
        // Signers stay signers — they're the fuzz signer wallet, not
        // preloaded from mainnet.
        let rebind_lines: String = aliasable
            .iter()
            .map(|name| {
                let f = addr_field(name);
                format!(
                    "                    \"{name}\" => {{ {f} = pk; }},\n"
                )
            })
            .collect();
        // User-token-account creation: manifest can declare
        // `user_token_accounts: [{account_alias, mint_alias,
        // signer_alias, amount}]`. Each entry creates an SPL TokenAccount
        // at `a_<account_alias>` (owned by the fuzz signer for
        // `signer_alias`, mint from `a_<mint_alias>`), so instructions
        // that read the user's balance find real bytes instead of an
        // empty placeholder — the R1 §5 "ok: 0" gap for movement calls.
        let user_ata_lines: String = aliasable
            .iter()
            .map(|name| {
                let f = addr_field(name);
                format!(
                    "                                \"{name}\" => {f},\n"
                )
            })
            .collect();
        // Signer lookups by name so `signer_alias` resolves to the
        // fuzz keypair's pubkey.
        let signer_lookup: String = self
            .roles
            .iter()
            .filter(|(_, r)| matches!(r, Role::Signer))
            .map(|(n, _)| {
                let f = kp_field(n);
                format!(
                    "                                \"{n}\" => {f}.pubkey(),\n"
                )
            })
            .collect();
        format!(
            "\n        // -- Mainnet snapshot preload (R2b).\n\
             \x20       // Non-trivial state (config / pool / mints / vaults) that the\n\
             \x20       // Anchor IDL cannot express is loaded here from mainnet dumps.\n\
             \x20       // Manifest: {{ \"aliases\": {{ \"<idl_name>\": \"<mainnet_pubkey>\" }} }}.\n\
             \x20       // Snapshot files: `snapshot_<pubkey>.json` per\n\
             \x20       // `solana account <pk> --output json`. Absent dir → no-op.\n\
             \x20       let snap_root = std::path::Path::new(SNAPSHOT_DIR);\n\
             \x20       if snap_root.exists() {{\n\
             \x20           let manifest_path = snap_root.join(SNAPSHOT_MANIFEST);\n\
             \x20           if let Ok(txt) = std::fs::read_to_string(&manifest_path) {{\n\
             \x20               if let Ok(m) = serde_json::from_str::<serde_json::Value>(&txt) {{\n\
             \x20                   if let Some(aliases) = m.get(\"aliases\").and_then(|v| v.as_object()) {{\n\
             \x20                       for (idl_name, pk_val) in aliases {{\n\
             \x20                           let pk_str = match pk_val.as_str() {{ Some(s) => s, None => continue }};\n\
             \x20                           let pk = match Pubkey::from_str(pk_str) {{ Ok(p) => p, Err(_) => continue }};\n\
             \x20                           // ALWAYS rebind the fixture's local for this IDL name\n\
             \x20                           // (even when the snapshot file is missing) — sibling\n\
             \x20                           // accounts like `token_0_program` need to resolve to the\n\
             \x20                           // real SPL Token program id (already in the SVM), not a\n\
             \x20                           // random placeholder pubkey.\n\
             \x20                           match idl_name.as_str() {{\n\
             {rebind_lines}\
             \x20                               _ => {{}},\n\
             \x20                           }}\n\
             \x20                           // Best-effort snapshot preload of the account bytes\n\
             \x20                           // themselves. Missing file → skip (already rebound above).\n\
             \x20                           let snap_path = snap_root.join(format!(\"snapshot_{{}}.json\", pk_str));\n\
             \x20                           let snap_txt = match std::fs::read_to_string(&snap_path) {{ Ok(t) => t, Err(_) => continue }};\n\
             \x20                           let snap: serde_json::Value = match serde_json::from_str(&snap_txt) {{ Ok(v) => v, Err(_) => continue }};\n\
             \x20                           let acct = snap.get(\"account\").unwrap_or(&snap);\n\
             \x20                           let owner_str = match acct.get(\"owner\").and_then(|v| v.as_str()) {{ Some(s) => s, None => continue }};\n\
             \x20                           let owner_pk = match Pubkey::from_str(owner_str) {{ Ok(p) => p, Err(_) => continue }};\n\
             \x20                           let lamports = acct.get(\"lamports\").and_then(|v| v.as_u64()).unwrap_or(0);\n\
             \x20                           let data_bytes: Vec<u8> = if let Some(arr) = acct.get(\"data\").and_then(|v| v.as_array()) {{\n\
             \x20                               // `solana account --output json` emits [payload, encoding].\n\
             \x20                               // Accept either order for robustness.\n\
             \x20                               if arr.len() >= 2 {{\n\
             \x20                                   let a0 = arr[0].as_str().unwrap_or(\"\");\n\
             \x20                                   let a1 = arr[1].as_str().unwrap_or(\"\");\n\
             \x20                                   let (payload, encoded) = if a1 == \"base64\" || a1 == \"base58\" {{ (a0, a1) }} else {{ (a1, a0) }};\n\
             \x20                                   match encoded {{\n\
             \x20                                       \"base64\" => base64_decode(payload),\n\
             \x20                                       _ => Vec::new(),\n\
             \x20                                   }}\n\
             \x20                               }} else {{ Vec::new() }}\n\
             \x20                           }} else if let Some(bytes) = acct.get(\"data\").and_then(|v| v.as_str()) {{\n\
             \x20                               base64_decode(bytes)\n\
             \x20                           }} else {{ Vec::new() }};\n\
             \x20                           if acct.get(\"executable\").and_then(|v| v.as_bool()).unwrap_or(false) {{\n\
             \x20                               // Executable programs are loaded via add_program, not\n\
             \x20                               // set_account. Skip — the SVM already has SPL Token, ATA,\n\
             \x20                               // System, etc. as built-ins.\n\
             \x20                               continue;\n\
             \x20                           }}\n\
             \x20                           let _ = ctx\n\
             \x20                               .create_account()\n\
             \x20                               .pubkey(pk)\n\
             \x20                               .owner(owner_pk)\n\
             \x20                               .lamports(if lamports > 0 {{ lamports }} else {{ INITIAL_BALANCE }})\n\
             \x20                               .data(&data_bytes)\n\
             \x20                               .create();\n\
             \x20                       }}\n\
             \x20                       // -- User SPL token accounts (R2b, per §5 item 1)\n\
             \x20                       if let Some(utas) = m.get(\"user_token_accounts\").and_then(|v| v.as_array()) {{\n\
             \x20                           for uta in utas {{\n\
             \x20                               let acct_alias = match uta.get(\"account_alias\").and_then(|v| v.as_str()) {{ Some(s) => s, None => continue }};\n\
             \x20                               let mint_alias = match uta.get(\"mint_alias\").and_then(|v| v.as_str()) {{ Some(s) => s, None => continue }};\n\
             \x20                               let signer_alias = match uta.get(\"signer_alias\").and_then(|v| v.as_str()) {{ Some(s) => s, None => continue }};\n\
             \x20                               let amount = uta.get(\"amount\").and_then(|v| v.as_u64()).unwrap_or(1_000_000_000);\n\
             \x20                               let acct_pk = match acct_alias {{\n\
             {user_ata_lines}\
             \x20                                   _ => continue,\n\
             \x20                               }};\n\
             \x20                               let mint_pk = match mint_alias {{\n\
             {user_ata_lines}\
             \x20                                   _ => continue,\n\
             \x20                               }};\n\
             \x20                               let signer_pk = match signer_alias {{\n\
             {signer_lookup}\
             \x20                                   _ => continue,\n\
             \x20                               }};\n\
             \x20                               let _ = ctx\n\
             \x20                                   .create_token_account()\n\
             \x20                                   .pubkey(acct_pk)\n\
             \x20                                   .mint(mint_pk)\n\
             \x20                                   .token_owner(signer_pk)\n\
             \x20                                   .amount(amount)\n\
             \x20                                   .create();\n\
             \x20                           }}\n\
             \x20                       }}\n\
             \x20                   }}\n\
             \x20               }}\n\
             \x20           }}\n\
             \x20       }}"
        )
    }

    /// R3 closure A: baseline-seeding block. For each SupplyMint spec,
    /// reads the just-preloaded state account + SPL Mint and records the
    /// initial `mint.supply - state.<supply_field>` drift into a local
    /// (later stashed into the fixture struct). For each FeeMonotone
    /// spec, reads the state account and records the initial value so
    /// `prev_<field>` starts from the loaded on-chain value.
    ///
    /// Emitted code is defensive: any read failure leaves the local at
    /// its default (`None` for SupplyMint → helper skips; `0` for
    /// FeeMonotone → helper starts from 0, matches R2b behavior). No
    /// panics on cold-start against an absent snapshot dir.
    #[allow(clippy::type_complexity)]
    fn baseline_seed_block(
        &self,
        supplymint_baselines: &[(String, String, String, String, String)],
        monotone_fields: &[(String, String, String)],
    ) -> String {
        if supplymint_baselines.is_empty() && monotone_fields.is_empty() {
            return String::new();
        }
        let mut out = String::new();
        out.push_str(
            "\n        // -- R3 closure A: seed baselines from just-preloaded state.\n\
             \x20       // SupplyMint drift: current invariant becomes\n\
             \x20       //   (mint.supply - state.<field>) == baseline_delta   (drift-preserving),\n\
             \x20       // instead of strict equality (which fires on every seed against\n\
             \x20       // CP-Swap's design-lockup MINIMUM_LIQUIDITY offset). FeeMonotone:\n\
             \x20       // prev_<field> starts at the on-chain value so the assertion is\n\
             \x20       // non-trivial from step 0.\n",
        );
        for (state_ty, _state_name, supply_field, mint_name, id) in supplymint_baselines {
            let ty_ident = capitalize_snake(state_ty);
            let field_id = sanitize_ident(supply_field);
            let mint_addr = format!("a_{}", sanitize_ident(mint_name));
            // Find the state account addr for this spec (any spec in the
            // SupplyMint tuple carries the same state_account_name — we
            // pass it in the tuple; use it here).
            let state_addr = format!("a_{}", sanitize_ident(_state_name));
            out.push_str(&format!(
                "        let baseline_local_{id}: Option<i128> = {{\n\
                 \x20           let acct = ctx.read_anchor_account::<{ty_ident}>(&{state_addr}).ok();\n\
                 \x20           let mint = ctx.svm.get_account(&{mint_addr}).and_then(|a| spl_token::state::Mint::unpack(&a.data).ok());\n\
                 \x20           match (acct, mint) {{\n\
                 \x20               (Some(s), Some(m)) => Some(m.supply as i128 - s.{field_id} as i128),\n\
                 \x20               _ => None,\n\
                 \x20           }}\n\
                 \x20       }};\n"
            ));
        }
        for (state_ty, field, state_name) in monotone_fields {
            let ty_ident = capitalize_snake(state_ty);
            let field_id = sanitize_ident(field);
            let state_addr = format!("a_{}", sanitize_ident(state_name));
            out.push_str(&format!(
                "        let baseline_prev_{field_id}: u64 = ctx\n\
                 \x20           .read_anchor_account::<{ty_ident}>(&{state_addr})\n\
                 \x20           .map(|s| s.{field_id})\n\
                 \x20           .unwrap_or(0);\n"
            ));
        }
        out
    }

    /// Generated standalone bindings: instruction data + account-meta
    /// structs for every supported instruction, plus the state-account
    /// struct closure for `state_types` (usually just the tracked type).
    fn bindings(&self, state_types: &[String]) -> Result<String, EmitError> {
        let mut out = String::new();
        out.push_str(
            "// ---------------------------------------------------------------------------\n\
             // IDL-derived client bindings (generated).\n\
             // ---------------------------------------------------------------------------\n",
        );
        for ix in &self.supported {
            out.push('\n');
            out.push_str(&self.instruction_bindings(ix));
        }
        for ty in state_types {
            out.push('\n');
            out.push_str(&self.state_type_bindings(ty)?);
        }
        Ok(out)
    }

    fn instruction_bindings(&self, ix: &Instruction) -> String {
        let camel = capitalize_snake(&ix.name);
        let data_name = format!("Ix{camel}");
        let accounts_name = format!("Ix{camel}Accounts");
        let disc = byte_list(&ix.discriminator);

        // Data struct.
        let mut fields = String::new();
        let mut ser = String::new();
        for arg in &ix.arg_defs {
            let ident = sanitize_ident(&arg.name);
            let rust_ty = rust_type(&arg.ty)
                .expect("supported instructions carry scalar/pubkey args only");
            fields.push_str(&format!("    pub {ident}: {rust_ty},\n"));
            ser.push_str(&format!(
                "        anchor_lang::AnchorSerialize::serialize(&self.{ident}, writer)?;\n"
            ));
        }
        // Account-meta struct.
        let mut acct_fields = String::new();
        let mut metas = String::new();
        for a in &ix.account_defs {
            let ident = sanitize_ident(&a.name);
            acct_fields.push_str(&format!("    pub {ident}: Pubkey,\n"));
            let ctor = if a.writable { "new" } else { "new_readonly" };
            let signer = if a.signer { "true" } else { "false" };
            metas.push_str(&format!(
                "            AccountMeta::{ctor}(self.{ident}, {signer}),\n"
            ));
        }
        format!
            ("/// `{}` instruction data (discriminator + args from the IDL).\n\
             pub struct {data_name} {{\n{fields}}}\n\
             impl anchor_lang::Discriminator for {data_name} {{\n\
             \x20   const DISCRIMINATOR: &'static [u8] = &{disc};\n\
             }}\n\
             impl anchor_lang::AnchorSerialize for {data_name} {{\n\
             \x20   fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {{\n\
             {ser}\x20       Ok(())\n\
             \x20   }}\n\
             }}\n\
             impl anchor_lang::InstructionData for {data_name} {{}}\n\
             \n\
             /// `{}` account metas (order + writable/signer flags from the IDL).\n\
             pub struct {accounts_name} {{\n{acct_fields}}}\n\
             impl anchor_lang::ToAccountMetas for {accounts_name} {{\n\
             \x20   fn to_account_metas(&self, _is_signer: Option<bool>) -> Vec<AccountMeta> {{\n\
             \x20       vec![\n{metas}\x20       ]\n\
             \x20   }}\n\
             }}\n",
            ix.name, ix.name
        )
    }

    /// Generate the struct (and its defined-type closure) for a state
    /// account, with manual `AnchorDeserialize` + `Discriminator`.
    fn state_type_bindings(&self, type_name: &str) -> Result<String, EmitError> {
        let mut ordered = Vec::new();
        let mut seen = BTreeSet::new();
        self.collect_type_closure(type_name, &mut ordered, &mut seen)?;

        let mut out = String::new();
        for name in ordered.iter().rev() {
            let tdef = self.type_def(name)?;
            check_layout_supported(tdef)?;
            match &tdef.body {
                TypeDefBody::Struct { fields } => {
                    out.push_str(&self.struct_binding(tdef, fields)?);
                }
                TypeDefBody::Enum { variants } => {
                    out.push_str(&enum_binding(&tdef.name, variants)?);
                }
            }
            out.push('\n');
        }
        // Discriminator only for the account type itself (nested types
        // are read inline, no discriminator prefix).
        let disc = self
            .surface
            .account_types
            .iter()
            .find(|a| a.name == type_name)
            .filter(|a| a.discriminator.len() == 8)
            .map(|a| a.discriminator.clone())
            .ok_or_else(|| {
                EmitError::MissingIdlData(format!(
                    "no 8-byte discriminator for account type `{type_name}` in the IDL \
                     `accounts` array — `read_anchor_account` cannot verify reads"
                ))
            })?;
        out.push_str(&format!(
            "impl anchor_lang::Discriminator for {} {{\n\
             \x20   const DISCRIMINATOR: &'static [u8] = &{};\n\
             }}\n",
            capitalize_snake(type_name),
            byte_list(&disc)
        ));
        Ok(out)
    }

    fn collect_type_closure(
        &self,
        name: &str,
        ordered: &mut Vec<String>,
        seen: &mut BTreeSet<String>,
    ) -> Result<(), EmitError> {
        if !seen.insert(name.to_string()) {
            return Ok(());
        }
        ordered.push(name.to_string());
        let tdef = self.type_def(name)?;
        if let TypeDefBody::Struct { fields } = &tdef.body {
            for f in fields {
                for dep in defined_deps(&f.ty) {
                    self.collect_type_closure(&dep, ordered, seen)?;
                }
            }
        }
        Ok(())
    }

    fn type_def(&self, name: &str) -> Result<&'a TypeDef, EmitError> {
        self.surface
            .type_defs
            .iter()
            .find(|t| t.name == name)
            .ok_or_else(|| {
                EmitError::MissingIdlData(format!(
                    "type `{name}` is referenced but has no layout in the IDL `types` array"
                ))
            })
    }

    fn struct_binding(&self, tdef: &TypeDef, fields: &[FieldDef]) -> Result<String, EmitError> {
        let name = capitalize_snake(&tdef.name);
        let mut field_lines = String::new();
        let mut de = String::new();
        for f in fields {
            let ident = sanitize_ident(&f.name);
            let rust_ty = rust_type(&f.ty).ok_or_else(|| {
                EmitError::Unsupported(format!(
                    "field `{}.{}` has IDL type {:?}, which this emitter cannot map to a \
                     Rust type",
                    tdef.name, f.name, f.ty
                ))
            })?;
            field_lines.push_str(&format!("    pub {ident}: {rust_ty},\n"));
            de.push_str(&format!(
                "            {ident}: anchor_lang::AnchorDeserialize::deserialize_reader(reader)?,\n"
            ));
        }
        let zc_note = if is_zero_copy(tdef) {
            "// Zero-copy account with `repr(C, packed)`: byte layout matches\n\
             // sequential field reading (no alignment padding), so the borsh-\n\
             // style reader below is layout-exact.\n"
        } else {
            ""
        };
        Ok(format!(
            "/// `{}` state account layout, from the IDL `types` array.\n\
             {zc_note}pub struct {name} {{\n{field_lines}}}\n\
             impl anchor_lang::AnchorDeserialize for {name} {{\n\
             \x20   fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {{\n\
             \x20       Ok(Self {{\n{de}\x20       }})\n\
             \x20   }}\n\
             }}\n",
            tdef.name
        ))
    }

    // -- address resolution --------------------------------------------------

    /// `setup()` code that binds every named account to an address, in
    /// dependency order. Also returns the ordered (field, decl) lists
    /// for the fixture struct.
    fn resolution_code(&self) -> (String, Vec<(String, String)>) {
        let mut code = String::new();
        let mut fields: Vec<(String, String)> = Vec::new();

        // 1. Fixed addresses.
        let mut any_fixed = false;
        for (name, role) in &self.roles {
            if let Role::Fixed(addr) = role {
                if !any_fixed {
                    code.push_str("        // Addresses the IDL pins.\n");
                    any_fixed = true;
                }
                let f = addr_field(name);
                code.push_str(&format!(
                    "        let {f} = Pubkey::from_str(\"{addr}\")\n\
                     \x20           .expect(\"IDL-pinned address is valid base58\");\n"
                ));
                fields.push((f, "Pubkey".into()));
            }
        }

        // 2. Signers — one funded Keypair per IDL signer name.
        let mut any_signer = false;
        for (name, role) in &self.roles {
            if matches!(role, Role::Signer) {
                if !any_signer {
                    code.push_str(
                        "\n        // One funded Keypair per IDL signer account name.\n",
                    );
                    any_signer = true;
                }
                let f = kp_field(name);
                code.push_str(&format!(
                    "        let {f} = Rc::new(Keypair::new());\n\
                     \x20       ctx.create_account()\n\
                     \x20           .pubkey({f}.pubkey())\n\
                     \x20           .lamports(INITIAL_BALANCE)\n\
                     \x20           .owner(system_program::ID)\n\
                     \x20           .create()\n\
                     \x20           .unwrap();\n"
                ));
                fields.push((f, "Rc<Keypair>".into()));
            }
        }

        // 3. Placeholders — the IDL constrains nothing about these
        // accounts; one stable Pubkey per name keeps every reference
        // pointing at the same address.
        let mut any_ph = false;
        for (name, role) in &self.roles {
            if matches!(role, Role::Placeholder) {
                if !any_ph {
                    code.push_str(
                        "\n        // The IDL carries no address, seeds, or signer flag for\n\
                         \x20       // these accounts — one stable placeholder Pubkey per name.\n",
                    );
                    any_ph = true;
                }
                let f = addr_field(name);
                code.push_str(&format!(
                    "        let {f} = Keypair::new().pubkey();\n"
                ));
                fields.push((f, "Pubkey".into()));
            }
        }

        // 4. Stable PDAs, in dependency order.
        let mut emitted: BTreeSet<String> = self
            .roles
            .iter()
            .filter(|(_, r)| !matches!(r, Role::StablePda(_)))
            .map(|(n, _)| n.clone())
            .collect();
        let mut pending: Vec<(&String, &PdaDef)> = self
            .roles
            .iter()
            .filter_map(|(n, r)| match r {
                Role::StablePda(pda) => Some((n, pda)),
                _ => None,
            })
            .collect();
        let mut any_pda = false;
        while !pending.is_empty() {
            let mut progressed = false;
            let mut still = Vec::new();
            for (name, pda) in pending {
                let deps_ready = pda.seeds.iter().all(|s| match s {
                    SeedDef::Account { path } => emitted.contains(path),
                    _ => true,
                });
                if deps_ready {
                    if !any_pda {
                        code.push_str(
                            "\n        // PDA addresses, derived from the IDL seed specs.\n",
                        );
                        any_pda = true;
                    }
                    let f = addr_field(name);
                    code.push_str(&format!(
                        "        let {f} = {};\n",
                        self.pda_expr(pda, false, &BTreeMap::new())
                    ));
                    fields.push((f, "Pubkey".into()));
                    emitted.insert(name.clone());
                    progressed = true;
                } else {
                    still.push((name, pda));
                }
            }
            pending = still;
            if !progressed {
                // Cycles can't happen for seeds that passed the support
                // check; defensive break keeps this total.
                break;
            }
        }

        (code, fields)
    }

    /// Expression for the address of account `name` as instruction
    /// `ix` sees it. `in_setup` controls `self.` prefixing;
    /// `arg_scope` maps arg names available as local bindings (for
    /// arg-seeded PDAs).
    fn account_expr(
        &self,
        ix: &Instruction,
        name: &str,
        in_setup: bool,
        arg_scope: &BTreeMap<String, String>,
    ) -> String {
        // Per-instruction arg-seeded PDA: derive inline with this
        // call's argument values.
        if let Some(def) = ix.account_defs.iter().find(|a| a.name == name) {
            if let Some(pda) = &def.pda {
                if pda_has_arg_seed(pda) {
                    return self.pda_expr_inner(ix, pda, in_setup, arg_scope);
                }
            }
        }
        self.role_expr(name, in_setup)
    }

    fn role_expr(&self, name: &str, in_setup: bool) -> String {
        let prefix = if in_setup { "" } else { "self." };
        match self.roles.get(name) {
            Some(Role::Signer) => format!("{prefix}{}.pubkey()", kp_field(name)),
            _ => format!("{prefix}{}", addr_field(name)),
        }
    }

    /// Derivation for a stable PDA (no arg seeds).
    fn pda_expr(
        &self,
        pda: &PdaDef,
        in_setup_locals: bool,
        arg_scope: &BTreeMap<String, String>,
    ) -> String {
        // In setup, refs are local `let` bindings (no self.).
        let seeds: Vec<String> = pda
            .seeds
            .iter()
            .map(|s| self.seed_expr(s, true, arg_scope))
            .collect();
        let prog = self.pda_program_expr(pda, true);
        let _ = in_setup_locals;
        format!(
            "Pubkey::find_program_address(&[{}], {}).0",
            seeds.join(", "),
            prog
        )
    }

    fn pda_expr_inner(
        &self,
        _ix: &Instruction,
        pda: &PdaDef,
        in_setup: bool,
        arg_scope: &BTreeMap<String, String>,
    ) -> String {
        let seeds: Vec<String> = pda
            .seeds
            .iter()
            .map(|s| self.seed_expr(s, in_setup, arg_scope))
            .collect();
        let prog = self.pda_program_expr(pda, in_setup);
        format!(
            "Pubkey::find_program_address(&[{}], {}).0",
            seeds.join(", "),
            prog
        )
    }

    fn seed_expr(
        &self,
        seed: &SeedDef,
        in_setup: bool,
        arg_scope: &BTreeMap<String, String>,
    ) -> String {
        match seed {
            SeedDef::Const { value } => format!("&{}[..]", byte_list(value)),
            SeedDef::Account { path } => {
                format!("{}.as_ref()", self.role_expr(path, in_setup))
            }
            SeedDef::Arg { path } => {
                let local = arg_scope
                    .get(path)
                    .cloned()
                    .unwrap_or_else(|| sanitize_ident(path));
                format!("&{local}.to_le_bytes()[..]")
            }
        }
    }

    fn pda_program_expr(&self, pda: &PdaDef, in_setup: bool) -> String {
        match &pda.program {
            None => {
                if in_setup {
                    "&program_id".into()
                } else {
                    "&self.program_id".into()
                }
            }
            Some(SeedDef::Const { value }) if value.len() == 32 => {
                let mut arr = String::from("[");
                for (i, b) in value.iter().enumerate() {
                    if i > 0 {
                        arr.push_str(", ");
                    }
                    arr.push_str(&b.to_string());
                }
                arr.push(']');
                format!("&Pubkey::new_from_array({arr})")
            }
            Some(SeedDef::Account { path }) => format!("&{}", self.role_expr(path, in_setup)),
            // Anything else was rejected by the support check.
            Some(other) => format!("&program_id /* unsupported pda program {other:?} */"),
        }
    }

    // -- instruction call rendering -----------------------------------------

    /// A `.call(..).accounts(..).signers(..)` chain for `ix`.
    /// `arg_exprs` maps arg name -> Rust expression for its value.
    fn call_chain(
        &self,
        ix: &Instruction,
        arg_exprs: &BTreeMap<String, String>,
        in_setup: bool,
        attacker_sub: Option<&str>,
        indent: &str,
    ) -> String {
        let camel = capitalize_snake(&ix.name);
        let ctx = if in_setup { "ctx" } else { "self.ctx" };
        let pid = if in_setup {
            "program_id"
        } else {
            "self.program_id"
        };
        let mut data_fields = String::new();
        for arg in &ix.arg_defs {
            let ident = sanitize_ident(&arg.name);
            let expr = arg_exprs
                .get(&arg.name)
                .cloned()
                .unwrap_or_else(|| ident.clone());
            if expr == ident {
                data_fields.push_str(&format!("{ident}, "));
            } else {
                data_fields.push_str(&format!("{ident}: {expr}, "));
            }
        }
        let data_fields = data_fields.trim_end().trim_end_matches(',').to_string();
        let data_body = if data_fields.is_empty() {
            String::new()
        } else {
            format!(" {data_fields} ")
        };

        let arg_scope: BTreeMap<String, String> = ix
            .arg_defs
            .iter()
            .map(|a| {
                (
                    a.name.clone(),
                    arg_exprs
                        .get(&a.name)
                        .cloned()
                        .unwrap_or_else(|| sanitize_ident(&a.name)),
                )
            })
            .collect();

        let mut acct_fields = String::new();
        for a in &ix.account_defs {
            let ident = sanitize_ident(&a.name);
            let expr = if a.signer && attacker_sub.is_some() {
                // Attack probe: the signer slot carries the attacker's
                // key; every other account keeps its legitimate value
                // (PDA seeds still resolve with the legitimate keys).
                format!("{}.pubkey()", attacker_sub.unwrap())
            } else {
                self.account_expr(ix, &a.name, in_setup, &arg_scope)
            };
            acct_fields.push_str(&format!("{indent}        {ident}: {expr},\n"));
        }

        let signers = if let Some(attacker) = attacker_sub {
            format!("&[&{attacker}]")
        } else {
            let list: Vec<String> = ix
                .account_defs
                .iter()
                .filter(|a| a.signer)
                .map(|a| {
                    if in_setup {
                        format!("&*{}", kp_field(&a.name))
                    } else {
                        format!("&*self.{}", kp_field(&a.name))
                    }
                })
                .collect();
            format!("&[{}]", list.join(", "))
        };

        format!(
            "{ctx}\n\
             {indent}    .program({pid})\n\
             {indent}    .call(Ix{camel} {{{data_body}}})\n\
             {indent}    .accounts(Ix{camel}Accounts {{\n{acct_fields}{indent}    }})\n\
             {indent}    .signers({signers})\n\
             {indent}    .send()"
        )
    }

    /// Best-effort bootstrap calls in `setup()`: initializer-flavored
    /// instructions once each (failure tolerated — for a deployed
    /// program whose init preconditions live outside the IDL, these
    /// legitimately fail and every action stays a rejected probe).
    fn bootstrap_code(&self, seed_positive_moves: bool) -> String {
        let mut out = String::new();
        let init_like: Vec<&&Instruction> = self
            .supported
            .iter()
            .filter(|ix| is_init_flavored(&ix.name))
            .collect();
        if !init_like.is_empty() {
            out.push_str(
                "\n        // Best-effort state bootstrap: initializer-flavored\n\
                 \x20       // instructions once each, minimal argument values. Failures\n\
                 \x20       // are tolerated — preconditions the IDL cannot express (e.g.\n\
                 \x20       // pre-existing mints or config accounts) make them fail loudly\n\
                 \x20       // at run time, never silently at emit time.\n",
            );
            for ix in &init_like {
                let arg_exprs = bootstrap_args(ix, 1);
                let (lets, exprs) = render_arg_lets(ix, &arg_exprs, "        ");
                out.push_str(&lets);
                out.push_str(&format!(
                    "        let _ = {};\n",
                    self.call_chain(ix, &exprs, true, None, "        ")
                ));
            }
        }
        if seed_positive_moves {
            let mut seeded = false;
            for mv in &self.candidate.emit_hints.ledger_moves {
                if !mv.add {
                    continue;
                }
                if let Some(ix) = self.supported.iter().find(|i| i.name == mv.action) {
                    if !seeded {
                        out.push_str(
                            "\n        // Seed state via the positive-direction movement\n\
                             \x20       // instruction(s) so a successful unauthorized probe is\n\
                             \x20       // observable as a state change, not a no-op.\n",
                        );
                        seeded = true;
                    }
                    let arg_exprs = bootstrap_args(ix, 1_000_000);
                    let (lets, exprs) = render_arg_lets(ix, &arg_exprs, "        ");
                    out.push_str(&lets);
                    out.push_str(&format!(
                        "        let _ = {};\n",
                        self.call_chain(ix, &exprs, true, None, "        ")
                    ));
                }
            }
        }
        out
    }

    fn setup_fn(
        &self,
        extra_field_inits: &str,
        seed_positive_moves: bool,
    ) -> (String, Vec<(String, String)>) {
        let (resolution, fields) = self.resolution_code();
        let bootstrap = self.bootstrap_code(seed_positive_moves);
        let mut self_fields = String::new();
        for (f, _) in &fields {
            self_fields.push_str(&format!("            {f},\n"));
        }
        let code = format!(
            "    pub fn setup() -> Self {{\n\
             \x20       let mut ctx = TestContext::new();\n\
             \x20       let program_id = Pubkey::from_str(PROGRAM_ID)\n\
             \x20           .expect(\"IDL `address` is valid base58\");\n\
             \x20       ctx.add_program(&program_id, PROGRAM_SO).unwrap();\n\
             \n\
             {resolution}{bootstrap}\n\
             \x20       Self {{\n\
             \x20           ctx,\n\
             \x20           program_id,\n\
             {self_fields}{extra_field_inits}\
             \x20       }}\n\
             \x20   }}"
        );
        (code, fields)
    }

    /// Normal fuzz action arm for `ix`. `ledger` renders the tracked-
    /// field bookkeeping mirror on success.
    fn action_arm(&self, ix: &Instruction, ledger: Option<(&str, &str, bool)>) -> String {
        let fn_name = format!("action_{}", sanitize_ident(&ix.name));
        let mut params = String::new();
        let mut arg_exprs = BTreeMap::new();
        for arg in &ix.arg_defs {
            let ident = sanitize_ident(&arg.name);
            if matches!(arg.ty, IdlType::Pubkey) {
                // Not fuzz-generated: bound to the calling signer's
                // pubkey (`instruction_supported` guarantees one).
                let signer = first_signer(ix).expect("supported => pubkey args have a signer");
                arg_exprs.insert(
                    arg.name.clone(),
                    format!("self.{}.pubkey()", kp_field(signer)),
                );
                continue;
            }
            let rust_ty = scalar_rust_type(&arg.ty).expect("supported => scalar or pubkey");
            match range_attr_for_arg(&arg.name, &arg.ty) {
                Some(r) => params.push_str(&format!(", #[range({r})] {ident}: {rust_ty}")),
                None => params.push_str(&format!(", {ident}: {rust_ty}")),
            }
        }
        let chain = self.call_chain(ix, &arg_exprs, false, None, "        ");
        let ledger_code = match ledger {
            Some((field, arg, add)) => {
                let op = if add { "saturating_add" } else { "saturating_sub" };
                let arg_ident = sanitize_ident(arg);
                format!(
                    "        if ok {{\n\
                     \x20           // Mirror the on-chain bookkeeping move (suggester hint:\n\
                     \x20           // `{arg}` moves the tracked field on `{}`).\n\
                     \x20           self.expected_{field} = self.expected_{field}.{op}({arg_ident} as u128);\n\
                     \x20       }}\n",
                    ix.name
                )
            }
            None => String::new(),
        };
        format!(
            "    pub fn {fn_name}(&mut self{params}) -> bool {{\n\
             \x20       let ok = {chain}\n\
             \x20           .map(|o| o.is_success())\n\
             \x20           .unwrap_or(false);\n\
             {ledger_code}\x20       ok\n\
             \x20   }}\n"
        )
    }

    /// Attacker probe arm (access_control): every signer slot swapped
    /// for a freshly-minted, funded Keypair; success trips the sticky
    /// flag.
    fn attack_arm(&self, ix: &Instruction) -> String {
        let fn_name = format!("action_attack_{}", sanitize_ident(&ix.name));
        let mut params = String::new();
        let mut arg_exprs = BTreeMap::new();
        for arg in &ix.arg_defs {
            let ident = sanitize_ident(&arg.name);
            if matches!(arg.ty, IdlType::Pubkey) {
                // In the attack probe the calling signer IS the
                // attacker, so the pubkey arg carries the attacker's
                // key — exactly what a real privilege-escalation call
                // would pass.
                arg_exprs.insert(arg.name.clone(), "attacker.pubkey()".to_string());
                continue;
            }
            let rust_ty = scalar_rust_type(&arg.ty).expect("supported => scalar or pubkey");
            match range_attr_for_arg(&arg.name, &arg.ty) {
                Some(r) => params.push_str(&format!(", #[range({r})] {ident}: {rust_ty}")),
                None => params.push_str(&format!(", {ident}: {rust_ty}")),
            }
        }
        let chain = self.call_chain(ix, &arg_exprs, false, Some("attacker"), "        ");
        format!(
            "    /// Attacker arm — probes `{}` with a freshly-minted Keypair\n\
             \x20   /// in every signer slot. All other accounts (including PDA\n\
             \x20   /// derivations) keep their legitimate values, so the probe\n\
             \x20   /// targets the real state. A correct program rejects the call;\n\
             \x20   /// success trips the sticky flag and the invariant fails.\n\
             \x20   pub fn {fn_name}(&mut self{params}) -> bool {{\n\
             \x20       let attacker = Keypair::new();\n\
             \x20       // Fund the attacker so a missing signer-check is the only\n\
             \x20       // way the call can succeed.\n\
             \x20       let _ = self.ctx\n\
             \x20           .create_account()\n\
             \x20           .pubkey(attacker.pubkey())\n\
             \x20           .lamports(INITIAL_BALANCE)\n\
             \x20           .owner(system_program::ID)\n\
             \x20           .create();\n\
             \x20       let attempted = {chain}\n\
             \x20           .map(|o| o.is_success())\n\
             \x20           .unwrap_or(false);\n\
             \x20       if attempted {{\n\
             \x20           self.unauthorized_success_observed = true;\n\
             \x20       }}\n\
             \x20       // Always return true so the fuzzer keeps generating actions.\n\
             \x20       true\n\
             \x20   }}\n",
            ix.name
        )
    }

    /// Bind the tracked account type to an instruction-account name.
    ///
    /// The Anchor 0.30+ IDL does NOT link instruction accounts to
    /// state-account types, so the binding is snake_case(name) equality
    /// — and it is VERIFIED at run time: `read_anchor_account` checks
    /// the account discriminator, so a wrong binding reads as "account
    /// absent" (invariant vacuous), never as wrong data.
    fn tracked_account_binding(&self) -> Result<String, EmitError> {
        let want = to_snake_case(&self.candidate.emit_hints.account_type);
        // 1. Exact snake_case name equality.
        for ix in &self.supported {
            for a in &ix.account_defs {
                if to_snake_case(&a.name) == want {
                    return Ok(a.name.clone());
                }
            }
        }
        // 2. Structurally-unique binding: if the program declares
        // exactly ONE state account type and exactly ONE account name
        // can carry program state (not a signer wallet, not an
        // IDL-pinned foreign address), the pair is determined by the
        // IDL itself — no name guessing. Still discriminator-verified
        // at run time.
        if self.surface.account_types.len() == 1 {
            let mut signers_or_fixed: BTreeSet<&str> = BTreeSet::new();
            let mut all: BTreeSet<&str> = BTreeSet::new();
            for ix in &self.supported {
                for a in &ix.account_defs {
                    all.insert(a.name.as_str());
                    if a.signer || a.address.is_some() {
                        signers_or_fixed.insert(a.name.as_str());
                    }
                }
            }
            let carriers: Vec<&str> = all
                .into_iter()
                .filter(|n| !signers_or_fixed.contains(n))
                .collect();
            if carriers.len() == 1 {
                return Ok(carriers[0].to_string());
            }
        }
        Err(EmitError::MissingIdlData(format!(
            "cannot bind account type `{}` to an instruction account: the Anchor IDL does \
             not link instruction accounts to account types, no instruction account is \
             named `{want}` (snake_case convention match), and the program's account-type/\
             state-carrier sets are not structurally unique. The fixture would not know \
             which address to read.",
            self.candidate.emit_hints.account_type
        )))
    }

    // -- class renderers -----------------------------------------------------

    fn render_balance(&self) -> Result<String, EmitError> {
        let h = &self.candidate.emit_hints;
        let account_ty_ident = capitalize_snake(&h.account_type);
        let field = sanitize_ident(&h.field);
        self.check_tracked_field_scalar()?;
        let tracked_name = self.tracked_account_binding()?;
        let fixture_name = self.fixture_name("");
        let invariant_fn = &self.candidate.name;

        let header = self.header(&format!(
            "// Fixture-side bookkeeping field: `expected_{field}: u128` — walked\n\
             // through every action and asserted against `{}.{}`\n\
             // after each step.",
            h.account_type, h.field
        ));
        let imports = self.imports_and_consts();
        let bindings = self.bindings(&[h.account_type.clone()])?;
        let (setup, fields) = self.setup_fn(
            &format!("            expected_{field}: 0,\n"),
            false,
        );

        let mut struct_fields = String::new();
        for (f, ty) in &fields {
            struct_fields.push_str(&format!("    {f}: {ty},\n"));
        }

        let mut arms = String::new();
        for ix in &self.supported {
            let ledger = h
                .ledger_moves
                .iter()
                .find(|m| m.action == ix.name)
                .map(|m| (field.as_str(), m.arg.as_str(), m.add));
            arms.push('\n');
            arms.push_str(&self.action_arm(ix, ledger));
        }

        let tracked_expr = self.role_expr(&tracked_name, false).replace("self.", "fixture.");

        Ok(format!(
            "{header}\n\
             {imports}\n\
             \n\
             {bindings}\n\
             #[derive(Clone)]\n\
             struct {fixture_name} {{\n\
             \x20   ctx: TestContext,\n\
             \x20   program_id: Pubkey,\n\
             {struct_fields}\
             \x20   /// Fixture-side ledger. Walked through every action; asserted\n\
             \x20   /// against on-chain `{account_ty}.{field_raw}` after each step.\n\
             \x20   expected_{field}: u128,\n\
             }}\n\
             \n\
             #[fuzz_fixture]\n\
             impl {fixture_name} {{\n\
             {setup}\n\
             {arms}}}\n\
             \n\
             // Balance-conservation invariant.\n\
             //\n\
             // After every action, the on-chain `{account_ty}.{field_raw}` must equal\n\
             // the fixture-side ledger (`expected_{field}`). Any drift indicates\n\
             // the program's bookkeeping has decoupled from the value it\n\
             // actually moved — the classic conservation violation.\n\
             #[invariant_test]\n\
             fn {invariant_fn}(fixture: &mut {fixture_name}) {{\n\
             \x20   // Discriminator-checked read; before the account exists (or if\n\
             \x20   // the name-binding was wrong) the read fails and the invariant\n\
             \x20   // is vacuous rather than a false alarm.\n\
             \x20   let acct: {account_ty_ident} = match fixture\n\
             \x20       .ctx\n\
             \x20       .read_anchor_account::<{account_ty_ident}>(&{tracked_expr})\n\
             \x20   {{\n\
             \x20       Ok(a) => a,\n\
             \x20       Err(_) => return,\n\
             \x20   }};\n\
             \x20   fuzz_assert_eq!(\n\
             \x20       acct.{field} as u128,\n\
             \x20       fixture.expected_{field},\n\
             \x20       \"{account_ty}.{field_raw} drift: on-chain={{}} expected={{}}\",\n\
             \x20       acct.{field},\n\
             \x20       fixture.expected_{field}\n\
             \x20   );\n\
             }}\n",
            account_ty = h.account_type,
            field_raw = h.field,
        ))
    }

    fn render_monotonic(&self) -> Result<String, EmitError> {
        let h = &self.candidate.emit_hints;
        let account_ty_ident = capitalize_snake(&h.account_type);
        let field = sanitize_ident(&h.field);
        self.check_tracked_field_scalar()?;
        let tracked_name = self.tracked_account_binding()?;
        let fixture_name = self.fixture_name("Monotonic");
        let invariant_fn = &self.candidate.name;

        let header = self.header(&format!(
            "// Fixture-side snapshot field: `last_seen_{field}: u128` — refreshed on\n\
             // every action AFTER the invariant check; the invariant asserts the\n\
             // current on-chain value is >= the previously-snapshotted value."
        ));
        let imports = self.imports_and_consts();
        let bindings = self.bindings(&[h.account_type.clone()])?;
        let (setup, fields) = self.setup_fn(
            &format!("            last_seen_{field}: 0,\n"),
            false,
        );

        let mut struct_fields = String::new();
        for (f, ty) in &fields {
            struct_fields.push_str(&format!("    {f}: {ty},\n"));
        }

        let mut arms = String::new();
        for ix in &self.supported {
            arms.push('\n');
            arms.push_str(&self.action_arm(ix, None));
        }

        let tracked_expr = self.role_expr(&tracked_name, false).replace("self.", "fixture.");

        Ok(format!(
            "{header}\n\
             {imports}\n\
             \n\
             {bindings}\n\
             #[derive(Clone)]\n\
             struct {fixture_name} {{\n\
             \x20   ctx: TestContext,\n\
             \x20   program_id: Pubkey,\n\
             {struct_fields}\
             \x20   /// Last-observed value of `{account_ty}.{field_raw}`. The invariant\n\
             \x20   /// asserts the current observation is >= this; it is then\n\
             \x20   /// refreshed to the current observation.\n\
             \x20   last_seen_{field}: u128,\n\
             }}\n\
             \n\
             #[fuzz_fixture]\n\
             impl {fixture_name} {{\n\
             {setup}\n\
             {arms}}}\n\
             \n\
             // Monotonic-accounting invariant.\n\
             //\n\
             // `{account_ty}.{field_raw}` is a lifetime/cumulative counter — a correct\n\
             // implementation never decreases it. The invariant snapshots the\n\
             // last observed value, asserts the current value is >= the\n\
             // snapshot, then refreshes the snapshot.\n\
             #[invariant_test]\n\
             fn {invariant_fn}(fixture: &mut {fixture_name}) {{\n\
             \x20   let acct: {account_ty_ident} = match fixture\n\
             \x20       .ctx\n\
             \x20       .read_anchor_account::<{account_ty_ident}>(&{tracked_expr})\n\
             \x20   {{\n\
             \x20       Ok(a) => a,\n\
             \x20       Err(_) => return,\n\
             \x20   }};\n\
             \x20   let current = acct.{field} as u128;\n\
             \x20   fuzz_assert_le!(\n\
             \x20       fixture.last_seen_{field},\n\
             \x20       current,\n\
             \x20       \"{account_ty}.{field_raw} regressed: snapshot={{}} current={{}}\",\n\
             \x20       fixture.last_seen_{field},\n\
             \x20       current\n\
             \x20   );\n\
             \x20   // Ratchet the snapshot forward.\n\
             \x20   fixture.last_seen_{field} = current;\n\
             }}\n",
            account_ty = h.account_type,
            field_raw = h.field,
        ))
    }

    fn render_access_control(&self) -> Result<String, EmitError> {
        let h = &self.candidate.emit_hints;
        let target_ix_name = h.action_names.first().cloned().ok_or_else(|| {
            EmitError::Unsupported(
                "access_control candidate carries no action_names — nothing to probe".into(),
            )
        })?;
        let target_ix = self
            .supported
            .iter()
            .find(|ix| ix.name == target_ix_name)
            .copied()
            .ok_or_else(|| {
                let reason = self
                    .excluded
                    .iter()
                    .find(|(n, _)| *n == target_ix_name)
                    .map(|(_, r)| r.clone())
                    .unwrap_or_else(|| "instruction not present in the IDL".into());
                EmitError::Unsupported(format!(
                    "access_control target instruction `{target_ix_name}` cannot be rendered: \
                     {reason}"
                ))
            })?;
        if !target_ix.account_defs.iter().any(|a| a.signer) {
            return Err(EmitError::MissingIdlData(format!(
                "instruction `{target_ix_name}` has no signer account in the IDL — an \
                 unauthorized-signer probe has no signer slot to attack"
            )));
        }

        let fixture_name = self.fixture_name("Access");
        let invariant_fn = &self.candidate.name;

        let header = self.header(
            "// The fixture bootstraps state, then in every `action_attack_*` arm\n\
             // probes the privileged instruction with a freshly-minted attacker\n\
             // `Keypair` signing instead of the legitimate signer(s). The\n\
             // invariant fails iff the program returned success on any attacker\n\
             // call.",
        );
        let imports = self.imports_and_consts();
        let bindings = self.bindings(&[])?;
        let (setup, fields) = self.setup_fn(
            "            unauthorized_success_observed: false,\n",
            true,
        );

        let mut struct_fields = String::new();
        for (f, ty) in &fields {
            struct_fields.push_str(&format!("    {f}: {ty},\n"));
        }

        let mut arms = String::new();
        for ix in &self.supported {
            arms.push('\n');
            arms.push_str(&self.action_arm(ix, None));
        }
        arms.push('\n');
        arms.push_str(&self.attack_arm(target_ix));

        Ok(format!(
            "{header}\n\
             {imports}\n\
             \n\
             {bindings}\n\
             #[derive(Clone)]\n\
             struct {fixture_name} {{\n\
             \x20   ctx: TestContext,\n\
             \x20   program_id: Pubkey,\n\
             {struct_fields}\
             \x20   /// Set to `true` on any successful attacker call. The invariant\n\
             \x20   /// asserts this stays `false` for the lifetime of the run.\n\
             \x20   unauthorized_success_observed: bool,\n\
             }}\n\
             \n\
             #[fuzz_fixture]\n\
             impl {fixture_name} {{\n\
             {setup}\n\
             {arms}}}\n\
             \n\
             // Access-control invariant.\n\
             //\n\
             // If the program ever accepted a `{target_ix_name}` call signed by an\n\
             // attacker instead of the legitimate signer(s), the sticky flag is\n\
             // `true` and this assertion fails.\n\
             #[invariant_test]\n\
             fn {invariant_fn}(fixture: &mut {fixture_name}) {{\n\
             \x20   fuzz_assert_eq!(\n\
             \x20       fixture.unauthorized_success_observed, false,\n\
             \x20       \"unauthorized {target_ix_name} succeeded against program {{}}\",\n\
             \x20       fixture.program_id\n\
             \x20   );\n\
             }}\n"
        ))
    }

    // -- relation_invariants (R2b) ------------------------------------------

    /// Render one fixture containing many `#[invariant_test]` fns —
    /// one per RelationSpec in the bundle. Setup is shared: PDAs +
    /// signers + placeholders + snapshot preload run once; each
    /// invariant reads the current live state and asserts.
    fn render_relation_bundle(&self) -> Result<String, EmitError> {
        let specs = &self.candidate.emit_hints.relation_specs;
        if specs.is_empty() {
            return Err(EmitError::Unsupported(
                "relation_invariants candidate has no relation_specs — nothing to render".into(),
            ));
        }

        // Filter specs to only those whose named accounts are actually
        // visible in this program's supported instructions (i.e. the
        // emitter produced a `Role` for them). This drops specs that
        // reference accounts on excluded instructions.
        let mut valid_specs = Vec::new();
        let mut skipped: Vec<(RelationSpec, String)> = Vec::new();
        let suppressed_list = &self.candidate.emit_hints.suppressed_specs;
        for spec in specs {
            // R3 closure B: allow the caller to suppress a validated
            // by-design finding (e.g. CP-Swap's MINIMUM_LIQUIDITY lockup
            // that makes RC-D fire on every seed on Raydium). Matched
            // against the helper fn name the spec would render as, either
            // exactly or as a prefix — so `helper_pool_state_matches_lp_mint_supply`
            // and `helper_pool_state_matches_lp` both work.
            let helper_name = self.helper_fn_name(spec);
            if suppressed_list
                .iter()
                .any(|s| helper_name == *s || helper_name.starts_with(s))
            {
                skipped.push((spec.clone(), format!("suppressed by --suppress-spec {helper_name}")));
                continue;
            }
            match self.check_spec_visible(spec) {
                Ok(()) => valid_specs.push(spec.clone()),
                Err(reason) => skipped.push((spec.clone(), reason.to_string())),
            }
        }
        if valid_specs.is_empty() {
            return Err(EmitError::Unsupported(format!(
                "no relation_specs are renderable — every candidate references accounts \
                 only present on excluded instructions ({} specs total, {} skipped)",
                specs.len(),
                skipped.len()
            )));
        }

        // Collect state account types we need bindings for (unique) —
        // ONLY from valid_specs (not raw specs), so a spec that got
        // filtered out (excluded instructions, suppressed by CLI, etc.)
        // does NOT force a state-type binding that would fail to render
        // (e.g. a non-packed zero-copy struct we can't emit).
        let mut needed_state_types: BTreeSet<String> = BTreeSet::new();
        for spec in &valid_specs {
            match spec {
                RelationSpec::Binding {
                    state_account_type, ..
                }
                | RelationSpec::SupplyMint {
                    state_account_type, ..
                }
                | RelationSpec::FeeMonotone {
                    state_account_type, ..
                } => {
                    needed_state_types.insert(state_account_type.clone());
                }
                RelationSpec::VaultBinding { .. } | RelationSpec::MintAuthority { .. } => {}
            }
        }
        let state_types: Vec<String> = needed_state_types.into_iter().collect();

        // The FeeMonotone specs need a `prev_<field>` fixture-side cache;
        // collect them so we can add fields + init.
        let mut monotone_fields: Vec<(String, String, String)> = Vec::new(); // (ty, field, state_account_name)
        let mut fee_seen: BTreeSet<(String, String)> = BTreeSet::new();
        for spec in &valid_specs {
            if let RelationSpec::FeeMonotone {
                state_account_type,
                state_account_name,
                field,
            } = spec
            {
                let key = (state_account_type.clone(), field.clone());
                if fee_seen.insert(key.clone()) {
                    monotone_fields.push((
                        state_account_type.clone(),
                        field.clone(),
                        state_account_name.clone(),
                    ));
                }
            }
        }
        let mut fee_field_decls = String::new();
        let mut fee_field_inits = String::new();
        for (_, f, _) in &monotone_fields {
            let id = sanitize_ident(f);
            fee_field_decls.push_str(&format!("    prev_{id}: u64,\n"));
            fee_field_inits.push_str(&format!("            prev_{id}: 0,\n"));
        }

        // R3 closure A: baseline delta seeded from snapshot state for
        // SupplyMint specs, so the invariant flips from strict-equality
        // (which fires on every seed against CP-Swap's design-lockup
        // 100-token drift, starving the corpus) to differential-drift-
        // preservation (`current_drift == baseline_drift_at_snapshot_load`).
        // Also enables per-spec `baseline_prev_<field>` for FeeMonotone so
        // `prev_<field>` starts from the loaded on-chain value rather than
        // 0 (which is trivially <=).
        let mut supplymint_baselines: Vec<(String, String, String, String, String)> = Vec::new();
        // (state_account_type, state_account_name, supply_field, mint_account_name, baseline_field_id)
        let mut supply_seen: BTreeSet<(String, String, String)> = BTreeSet::new();
        for spec in &valid_specs {
            if let RelationSpec::SupplyMint {
                state_account_type,
                state_account_name,
                supply_field,
                mint_account_name,
            } = spec
            {
                let key = (
                    state_account_type.clone(),
                    supply_field.clone(),
                    mint_account_name.clone(),
                );
                if supply_seen.insert(key.clone()) {
                    let baseline_id = format!(
                        "{}_{}",
                        sanitize_ident(supply_field),
                        sanitize_ident(mint_account_name)
                    );
                    supplymint_baselines.push((
                        state_account_type.clone(),
                        state_account_name.clone(),
                        supply_field.clone(),
                        mint_account_name.clone(),
                        baseline_id,
                    ));
                }
            }
        }
        let mut baseline_decls = String::new();
        let mut baseline_inits = String::new();
        for (_, _, _, _, id) in &supplymint_baselines {
            baseline_decls.push_str(&format!("    baseline_delta_{id}: Option<i128>,\n"));
            baseline_inits.push_str(&format!("            baseline_delta_{id}: None,\n"));
        }

        let fixture_name = self.fixture_name("Relations");
        let bundle_len = valid_specs.len();

        let header = self.header(&format!(
            "// R2b relation bundle: {bundle_len} invariant(s) emitted from IDL structure\n\
             // (RC-A / RC-D / RC-E / RC-G / RC-I). Setup preloads a mainnet snapshot when\n\
             // `SNAPSHOT_DIR` is present, so instructions execute against real,\n\
             // non-trivial state instead of a blank ledger."
        ));
        let imports = self.imports_and_consts();
        let bindings = self.bindings(&state_types)?;

        // Aliasable placeholder names — everything in the roles map
        // that isn't a signer keypair (those are the fresh fuzz signer,
        // never mainnet-preloaded).
        let aliasable: Vec<String> = self
            .roles
            .iter()
            .filter(|(_, r)| !matches!(r, Role::Signer))
            .map(|(n, _)| n.clone())
            .collect();

        // Custom setup: standard resolution+bootstrap, plus snapshot
        // preload + relation-bundle-specific field inits.
        let extra_inits = format!(
            "            snapshot_loaded: true,\n{fee_field_inits}{baseline_inits}"
        );
        let (mut setup, fields) = self.setup_fn(&extra_inits, false);
        // Inject the snapshot preload right before the `Self { ... }`
        // return so rebound locals apply.
        let preload = self.snapshot_preload(&aliasable);
        // R3 closure A: baseline-seeding block that reads the just-preloaded
        // state accounts and computes (a) the SupplyMint drift baseline and
        // (b) the FeeMonotone `prev_<field>` starting value from real on-chain
        // values. Runs INSIDE setup, after the snapshot preload, so `ctx` and
        // the (now-rebound) `a_<name>` locals resolve to the live mainnet
        // accounts. Empty when no baselines are needed.
        let baseline_seed = self.baseline_seed_block(&supplymint_baselines, &monotone_fields);
        if !preload.is_empty() || !baseline_seed.is_empty() {
            // Convert immutable placeholder lets to mutable so we can
            // rebind them from the snapshot manifest.
            setup = setup.replace("        let a_", "        let mut a_");
            let mut injected = String::new();
            if !preload.is_empty() {
                injected.push_str(&preload);
                injected.push('\n');
            }
            if !baseline_seed.is_empty() {
                // The baseline seeder emits `let mut baseline_..._<id> = None`
                // locals + reads that produce a value; the fixture struct
                // fields are populated in the `Self { ... }` return below.
                injected.push_str(&baseline_seed);
                injected.push('\n');
                // Rewrite `baseline_delta_<id>: None,` in extra_inits to pull
                // from the local we just computed.
                for (_, _, _, _, id) in &supplymint_baselines {
                    setup = setup.replace(
                        &format!("baseline_delta_{id}: None,"),
                        &format!("baseline_delta_{id}: baseline_local_{id},"),
                    );
                }
                for (_, f, _) in &monotone_fields {
                    let fid = sanitize_ident(f);
                    setup = setup.replace(
                        &format!("prev_{fid}: 0,"),
                        &format!("prev_{fid}: baseline_prev_{fid},"),
                    );
                }
            }
            setup = setup.replace(
                "\n        Self {\n",
                &format!("\n{injected}\n        Self {{\n"),
            );
        }

        let mut struct_fields = String::new();
        for (f, ty) in &fields {
            struct_fields.push_str(&format!("    {f}: {ty},\n"));
        }

        // Action arms so the fuzzer actually EXERCISES the program.
        let mut arms = String::new();
        for ix in &self.supported {
            arms.push('\n');
            arms.push_str(&self.action_arm(ix, None));
        }

        // Emit each relation spec as a regular helper fn, then a
        // single `#[invariant_test]` bundle fn that dispatches to all
        // of them in order. Crucible's `#[fuzz_fixture]` /
        // `#[invariant_test]` macros generate `main` for the fn whose
        // name matches the enabled feature — a single-invariant bundle
        // keeps that contract intact.
        let mut helpers = String::new();
        let mut bundle_body = String::new();
        for spec in &valid_specs {
            let (helper_fn_name, helper_code) = self.render_relation_helper(spec)?;
            helpers.push('\n');
            helpers.push_str(&helper_code);
            bundle_body.push_str(&format!("    {helper_fn_name}(fixture);\n"));
        }
        let invariant_fn_name = &self.candidate.name;
        let invariants = format!(
            "\n{helpers}\n/// Bundle invariant — dispatches to every derived relation check.\n\
             /// Any per-relation `fuzz_assert_*` failure fails the bundle.\n\
             #[invariant_test]\n\
             fn {invariant_fn_name}(fixture: &mut {fixture_name}) {{\n\
             {bundle_body}}}\n"
        );

        // Aggregate invariant that fails if too many actions still
        // rejected (surfaces the R1 execution-depth gap explicitly —
        // does NOT itself fail the run; comment-only signal).
        let mut skipped_note = String::new();
        if !skipped.is_empty() {
            skipped_note.push_str("//\n// Skipped relation specs (referenced accounts are only present\n// on excluded instructions):\n");
            for (spec, reason) in &skipped {
                skipped_note.push_str(&format!("//   - {spec:?}: {reason}\n"));
            }
        }

        Ok(format!(
            "{header}{skipped_note}\n\
             {imports}\n\
             // Pack imported from spl-token's re-export to match its\n\
             // Pack impls (avoids the multi-version solana-program-pack\n\
             // conflict when the harness pulls in both anchor-lang and\n\
             // spl-token from different registry versions).\n\
             use spl_token::solana_program::program_pack::Pack;\n\
             \n\
             // Small base64 decoder — the mainnet snapshot preload reads\n\
             // `[\"base64\", \"...\"]` account-data payloads from `solana account\n\
             // --output json` files. Keeps the harness free of a `base64` crate dep.\n\
             fn base64_decode(s: &str) -> Vec<u8> {{\n\
             \x20   const T: [i8; 256] = {{ let mut t = [-1i8; 256]; let a = b\"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/\"; let mut i = 0; while i < 64 {{ t[a[i] as usize] = i as i8; i += 1; }} t }};\n\
             \x20   let mut out = Vec::with_capacity(s.len() * 3 / 4);\n\
             \x20   let mut buf: u32 = 0; let mut bits: i32 = 0;\n\
             \x20   for &b in s.as_bytes() {{\n\
             \x20       if b == b'=' || b == b'\\n' || b == b'\\r' || b == b' ' {{ continue; }}\n\
             \x20       let v = T[b as usize]; if v < 0 {{ continue; }}\n\
             \x20       buf = (buf << 6) | (v as u32); bits += 6;\n\
             \x20       if bits >= 8 {{ bits -= 8; out.push((buf >> bits) as u8); buf &= (1u32 << bits) - 1; }}\n\
             \x20   }}\n\
             \x20   out\n\
             }}\n\
             \n\
             {bindings}\n\
             #[derive(Clone)]\n\
             struct {fixture_name} {{\n\
             \x20   ctx: TestContext,\n\
             \x20   program_id: Pubkey,\n\
             {struct_fields}\
             \x20   /// True iff the snapshot preload ran (kept for visibility).\n\
             \x20   snapshot_loaded: bool,\n\
             {fee_field_decls}{baseline_decls}\
             }}\n\
             \n\
             #[fuzz_fixture]\n\
             impl {fixture_name} {{\n\
             {setup}\n\
             {arms}}}\n\
             {invariants}"
        ))
    }

    /// Canonical helper-fn name for one spec — the token users match
    /// against with `--suppress-spec`. Kept in sync with the
    /// per-variant `format!("helper_...")` in `render_relation_helper`.
    fn helper_fn_name(&self, spec: &RelationSpec) -> String {
        match spec {
            RelationSpec::Binding {
                state_account_type, pubkey_field, ..
            } => format!(
                "helper_{}_{}_binding",
                to_snake_case(state_account_type),
                sanitize_ident(pubkey_field)
            ),
            RelationSpec::SupplyMint {
                state_account_type, mint_account_name, ..
            } => format!(
                "helper_{}_matches_{}_supply",
                to_snake_case(state_account_type),
                sanitize_ident(mint_account_name)
            ),
            RelationSpec::FeeMonotone { field, .. } => {
                format!("helper_{}_monotone", sanitize_ident(field))
            }
            RelationSpec::VaultBinding { vault_account_name, .. } => format!(
                "helper_{}_ownership_binding",
                sanitize_ident(vault_account_name)
            ),
            RelationSpec::MintAuthority { mint_account_name, .. } => format!(
                "helper_{}_mint_authority",
                sanitize_ident(mint_account_name)
            ),
        }
    }

    /// Check that every named account in the spec appears in the
    /// generator's roles map (i.e. is visible in some supported
    /// instruction). Returns a short reason on failure.
    fn check_spec_visible(&self, spec: &RelationSpec) -> Result<(), &'static str> {
        let names: Vec<&str> = match spec {
            RelationSpec::Binding {
                state_account_name,
                bound_account_name,
                ..
            } => vec![state_account_name.as_str(), bound_account_name.as_str()],
            RelationSpec::SupplyMint {
                state_account_name,
                mint_account_name,
                ..
            } => vec![state_account_name.as_str(), mint_account_name.as_str()],
            RelationSpec::FeeMonotone {
                state_account_name, ..
            } => vec![state_account_name.as_str()],
            RelationSpec::VaultBinding {
                vault_account_name,
                expected_mint_name,
                expected_owner_name,
                ..
            } => vec![
                vault_account_name.as_str(),
                expected_mint_name.as_str(),
                expected_owner_name.as_str(),
            ],
            RelationSpec::MintAuthority {
                mint_account_name,
                expected_authority_name,
            } => vec![
                mint_account_name.as_str(),
                expected_authority_name.as_str(),
            ],
        };
        for n in names {
            if !self.roles.contains_key(n) {
                return Err("account not present in any supported instruction");
            }
        }
        Ok(())
    }

    /// Render ONE relation spec as a plain helper fn `fn
    /// helper_<name>(fixture: &mut <Fixture>)` that runs its
    /// `fuzz_assert_*` calls. Bundle invariant_test dispatches to
    /// these; assertion failures propagate through Crucible's macro.
    fn render_relation_helper(
        &self,
        spec: &RelationSpec,
    ) -> Result<(String, String), EmitError> {
        let fixture_name = self.fixture_name("Relations");
        // Byte-level comparison for SPL Pubkey (a different concrete
        // type than solana_pubkey::Pubkey — they share the 32-byte
        // layout but Rust's type system rejects direct ==). We convert
        // via `.to_bytes()` so both sides are `[u8; 32]`.
        Ok(match spec {
            RelationSpec::Binding {
                state_account_type,
                state_account_name,
                pubkey_field,
                bound_account_name,
            } => {
                let ty_ident = capitalize_snake(state_account_type);
                let field_id = sanitize_ident(pubkey_field);
                let state_addr = self.role_expr(state_account_name, false).replace("self.", "fixture.");
                let bound_addr = self.role_expr(bound_account_name, false).replace("self.", "fixture.");
                let fn_name = format!(
                    "helper_{}_{}_binding",
                    to_snake_case(state_account_type),
                    field_id
                );
                let body = format!(
                    "/// RC-A (name binding): {state_account_type}.{pubkey_field} == {bound_account_name}.key\n\
                     fn {fn_name}(fixture: &mut {fixture_name}) {{\n\
                     \x20   let acct: {ty_ident} = match fixture.ctx.read_anchor_account::<{ty_ident}>(&{state_addr}) {{\n\
                     \x20       Ok(a) => a, Err(_) => return,\n\
                     \x20   }};\n\
                     \x20   fuzz_assert_eq!(acct.{field_id}, {bound_addr},\n\
                     \x20       \"{state_account_type}.{pubkey_field} decoupled from `{bound_account_name}`\");\n\
                     }}\n"
                );
                (fn_name, body)
            }
            RelationSpec::SupplyMint {
                state_account_type,
                state_account_name,
                supply_field,
                mint_account_name,
            } => {
                let ty_ident = capitalize_snake(state_account_type);
                let field_id = sanitize_ident(supply_field);
                let state_addr = self.role_expr(state_account_name, false).replace("self.", "fixture.");
                let mint_addr = self.role_expr(mint_account_name, false).replace("self.", "fixture.");
                let baseline_id = format!(
                    "{}_{}",
                    sanitize_ident(supply_field),
                    sanitize_ident(mint_account_name)
                );
                let fn_name = format!(
                    "helper_{}_matches_{}_supply",
                    to_snake_case(state_account_type),
                    sanitize_ident(mint_account_name)
                );
                // R3 closure A: when a baseline drift was seeded from the
                // snapshot at setup time, assert drift-preservation (the
                // program should never change one side without the other).
                // Fall back to strict-equality only when no baseline was
                // captured (blank SVM, no snapshot dir).
                let body = format!(
                    "/// RC-D (cross-program supply, drift-preserving): (mint.supply - state.{supply_field}) stable across actions.\n\
                     /// Falls back to strict equality when no baseline was seeded (no snapshot dir).\n\
                     fn {fn_name}(fixture: &mut {fixture_name}) {{\n\
                     \x20   let acct: {ty_ident} = match fixture.ctx.read_anchor_account::<{ty_ident}>(&{state_addr}) {{\n\
                     \x20       Ok(a) => a, Err(_) => return,\n\
                     \x20   }};\n\
                     \x20   let mint_bytes = match fixture.ctx.svm.get_account(&{mint_addr}) {{\n\
                     \x20       Some(a) if a.data.len() >= spl_token::state::Mint::LEN => a.data.clone(),\n\
                     \x20       _ => return,\n\
                     \x20   }};\n\
                     \x20   let mint = match spl_token::state::Mint::unpack(&mint_bytes) {{ Ok(m) => m, Err(_) => return }};\n\
                     \x20   match fixture.baseline_delta_{baseline_id} {{\n\
                     \x20       Some(base) => {{\n\
                     \x20           let current = mint.supply as i128 - acct.{field_id} as i128;\n\
                     \x20           fuzz_assert_eq!(current, base,\n\
                     \x20               \"{state_account_type}.{supply_field} vs SPL Mint({mint_account_name}).supply drift changed: baseline={{}} current={{}}\", base, current);\n\
                     \x20       }}\n\
                     \x20       None => {{\n\
                     \x20           fuzz_assert_eq!(acct.{field_id}, mint.supply,\n\
                     \x20               \"{state_account_type}.{supply_field}={{}} but SPL Mint({mint_account_name}).supply={{}}\", acct.{field_id}, mint.supply);\n\
                     \x20       }}\n\
                     \x20   }}\n\
                     }}\n"
                );
                (fn_name, body)
            }
            RelationSpec::FeeMonotone {
                state_account_type,
                state_account_name,
                field,
            } => {
                let ty_ident = capitalize_snake(state_account_type);
                let field_id = sanitize_ident(field);
                let state_addr = self.role_expr(state_account_name, false).replace("self.", "fixture.");
                let fn_name = format!("helper_{field_id}_monotone");
                let body = format!(
                    "/// RC-E (monotonicity): {state_account_type}.{field} never decreases between observations.\n\
                     fn {fn_name}(fixture: &mut {fixture_name}) {{\n\
                     \x20   let acct: {ty_ident} = match fixture.ctx.read_anchor_account::<{ty_ident}>(&{state_addr}) {{\n\
                     \x20       Ok(a) => a, Err(_) => return,\n\
                     \x20   }};\n\
                     \x20   fuzz_assert_le!(fixture.prev_{field_id}, acct.{field_id},\n\
                     \x20       \"{state_account_type}.{field} regressed: {{}} -> {{}}\", fixture.prev_{field_id}, acct.{field_id});\n\
                     \x20   fixture.prev_{field_id} = acct.{field_id};\n\
                     }}\n"
                );
                (fn_name, body)
            }
            RelationSpec::VaultBinding {
                vault_account_name,
                expected_mint_name,
                expected_owner_name,
            } => {
                let vault_addr = self.role_expr(vault_account_name, false).replace("self.", "fixture.");
                let mint_addr = self.role_expr(expected_mint_name, false).replace("self.", "fixture.");
                let owner_addr = self.role_expr(expected_owner_name, false).replace("self.", "fixture.");
                let fn_name = format!(
                    "helper_{}_ownership_binding",
                    sanitize_ident(vault_account_name)
                );
                let body = format!(
                    "/// RC-G (SPL vault): TokenAccount({vault_account_name}).mint == {expected_mint_name} AND .owner == {expected_owner_name}\n\
                     fn {fn_name}(fixture: &mut {fixture_name}) {{\n\
                     \x20   let bytes = match fixture.ctx.svm.get_account(&{vault_addr}) {{\n\
                     \x20       Some(a) if a.data.len() >= spl_token::state::Account::LEN => a.data.clone(),\n\
                     \x20       _ => return,\n\
                     \x20   }};\n\
                     \x20   let acct = match spl_token::state::Account::unpack(&bytes) {{ Ok(a) => a, Err(_) => return }};\n\
                     \x20   fuzz_assert_eq!(acct.mint.to_bytes(), {mint_addr}.to_bytes(),\n\
                     \x20       \"{vault_account_name}.mint drift (expected {expected_mint_name})\");\n\
                     \x20   fuzz_assert_eq!(acct.owner.to_bytes(), {owner_addr}.to_bytes(),\n\
                     \x20       \"{vault_account_name}.owner drift (expected {expected_owner_name})\");\n\
                     }}\n"
                );
                (fn_name, body)
            }
            RelationSpec::MintAuthority {
                mint_account_name,
                expected_authority_name,
            } => {
                let mint_addr = self.role_expr(mint_account_name, false).replace("self.", "fixture.");
                let auth_addr = self.role_expr(expected_authority_name, false).replace("self.", "fixture.");
                let fn_name = format!(
                    "helper_{}_mint_authority",
                    sanitize_ident(mint_account_name)
                );
                let body = format!(
                    "/// RC-I (mint authority): Mint::unpack({mint_account_name}).mint_authority == Some({expected_authority_name})\n\
                     fn {fn_name}(fixture: &mut {fixture_name}) {{\n\
                     \x20   let bytes = match fixture.ctx.svm.get_account(&{mint_addr}) {{\n\
                     \x20       Some(a) if a.data.len() >= spl_token::state::Mint::LEN => a.data.clone(),\n\
                     \x20       _ => return,\n\
                     \x20   }};\n\
                     \x20   let mint = match spl_token::state::Mint::unpack(&bytes) {{ Ok(m) => m, Err(_) => return }};\n\
                     \x20   let observed = mint.mint_authority.map(|k| k.to_bytes()).unwrap_or_default();\n\
                     \x20   fuzz_assert_eq!(observed, {auth_addr}.to_bytes(),\n\
                     \x20       \"{mint_account_name}.mint_authority drift (expected {expected_authority_name})\");\n\
                     }}\n"
                );
                (fn_name, body)
            }
        })
    }

    fn check_tracked_field_scalar(&self) -> Result<(), EmitError> {
        let h = &self.candidate.emit_hints;
        let tdef = self.type_def(&h.account_type)?;
        let fields = match &tdef.body {
            TypeDefBody::Struct { fields } => fields,
            TypeDefBody::Enum { .. } => {
                return Err(EmitError::Unsupported(format!(
                    "tracked account type `{}` is an enum, not a struct",
                    h.account_type
                )))
            }
        };
        let f = fields.iter().find(|f| f.name == h.field).ok_or_else(|| {
            EmitError::MissingIdlData(format!(
                "tracked field `{}.{}` is not in the IDL type layout",
                h.account_type, h.field
            ))
        })?;
        match f.ty {
            IdlType::U8 | IdlType::U16 | IdlType::U32 | IdlType::U64 | IdlType::U128 => Ok(()),
            ref other => Err(EmitError::Unsupported(format!(
                "tracked field `{}.{}` has type {other:?}; the u128 ledger encoding needs a \
                 scalar unsigned integer",
                h.account_type, h.field
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Support checks & role assignment.
// ---------------------------------------------------------------------------

/// Can this instruction be rendered faithfully as a fuzz action arm?
/// Returns the exact reason when it cannot.
fn instruction_supported(ix: &Instruction) -> Result<(), String> {
    if ix.discriminator.len() != 8 {
        return Err(format!(
            "no 8-byte instruction discriminator in the IDL (found {} bytes) — cannot \
             encode the call",
            ix.discriminator.len()
        ));
    }
    for arg in &ix.arg_defs {
        if matches!(arg.ty, IdlType::Pubkey) {
            // Pubkey args are bound to the calling signer's pubkey —
            // that needs a fixture-held signer on the instruction.
            if !ix.account_defs.iter().any(|a| a.signer) {
                return Err(format!(
                    "argument `{}` is a pubkey but `{}` has no signer account to \
                     bind it to",
                    arg.name, ix.name
                ));
            }
            continue;
        }
        if scalar_rust_type(&arg.ty).is_none() {
            return Err(format!(
                "argument `{}` has IDL type {:?}; the fuzzer generates scalar \
                 integers/bools (and fixes pubkey args to the calling signer)",
                arg.name, arg.ty
            ));
        }
    }
    let acct_names: BTreeSet<&str> = ix.account_defs.iter().map(|a| a.name.as_str()).collect();
    let arg_names: BTreeSet<&str> = ix.arg_defs.iter().map(|a| a.name.as_str()).collect();
    for a in &ix.account_defs {
        if a.optional {
            return Err(format!(
                "account `{}` is optional; optional-account encoding is not modeled",
                a.name
            ));
        }
        if a.signer && a.address.is_some() {
            return Err(format!(
                "signer `{}` is pinned to address {} — its private key is not derivable \
                 from the IDL, so the harness cannot produce this signature",
                a.name,
                a.address.as_deref().unwrap_or("?")
            ));
        }
        if let Some(pda) = &a.pda {
            for s in &pda.seeds {
                match s {
                    SeedDef::Const { .. } => {}
                    SeedDef::Account { path } => {
                        if path.contains('.') {
                            return Err(format!(
                                "PDA seed for `{}` references account data `{path}` — \
                                 resolving it needs field values the IDL does not carry",
                                a.name
                            ));
                        }
                        if path.starts_with("__unknown_seed_kind_") {
                            return Err(format!(
                                "PDA seed for `{}` has a seed kind this parser does not \
                                 model ({path})",
                                a.name
                            ));
                        }
                        if !acct_names.contains(path.as_str()) {
                            return Err(format!(
                                "PDA seed for `{}` references `{path}`, which is not an \
                                 account of this instruction",
                                a.name
                            ));
                        }
                    }
                    SeedDef::Arg { path } => {
                        if path.contains('.') {
                            return Err(format!(
                                "PDA seed for `{}` references nested argument `{path}` — \
                                 not modeled",
                                a.name
                            ));
                        }
                        if !arg_names.contains(path.as_str()) {
                            return Err(format!(
                                "PDA seed for `{}` references argument `{path}`, which \
                                 this instruction does not declare",
                                a.name
                            ));
                        }
                        let arg = ix.arg_defs.iter().find(|x| x.name == *path).unwrap();
                        if !matches!(
                            arg.ty,
                            IdlType::U8 | IdlType::U16 | IdlType::U32 | IdlType::U64 | IdlType::U128
                        ) {
                            return Err(format!(
                                "PDA seed for `{}` uses argument `{path}` of type {:?}; \
                                 only unsigned-integer seed args are encoded (little-endian \
                                 bytes, Anchor convention)",
                                a.name, arg.ty
                            ));
                        }
                    }
                }
            }
            if let Some(prog) = &pda.program {
                match prog {
                    SeedDef::Const { value } if value.len() == 32 => {}
                    SeedDef::Account { path } if acct_names.contains(path.as_str()) => {}
                    other => {
                        return Err(format!(
                            "PDA program for `{}` is {other:?}; only a 32-byte constant or \
                             a same-instruction account reference is modeled",
                            a.name
                        ));
                    }
                }
            }
        }
    }
    if !ix.account_defs.iter().any(|a| a.signer) {
        return Err(
            "no signer account — the harness has no keypair to pay/sign the transaction"
                .to_string(),
        );
    }
    Ok(())
}

fn assign_roles(supported: &[&Instruction]) -> BTreeMap<String, Role> {
    let mut roles: BTreeMap<String, Role> = BTreeMap::new();
    // 1. Fixed addresses win.
    for ix in supported {
        for a in &ix.account_defs {
            if let Some(addr) = &a.address {
                roles
                    .entry(a.name.clone())
                    .or_insert_with(|| Role::Fixed(addr.clone()));
            }
        }
    }
    // 2. Signers.
    for ix in supported {
        for a in &ix.account_defs {
            if a.signer {
                roles.entry(a.name.clone()).or_insert(Role::Signer);
            }
        }
    }
    // 3. Stable PDAs (no arg seeds) — fixed point over account refs.
    loop {
        let mut progressed = false;
        for ix in supported {
            for a in &ix.account_defs {
                if roles.contains_key(&a.name) {
                    continue;
                }
                if let Some(pda) = &a.pda {
                    if pda_has_arg_seed(pda) {
                        continue;
                    }
                    let refs_ok = pda.seeds.iter().all(|s| match s {
                        SeedDef::Account { path } => {
                            roles.contains_key(path)
                                || will_be_placeholder(path, supported, &roles)
                        }
                        _ => true,
                    });
                    if refs_ok {
                        roles.insert(a.name.clone(), Role::StablePda(pda.clone()));
                        progressed = true;
                    }
                }
            }
        }
        if !progressed {
            break;
        }
    }
    // 4. Everything else is a stable placeholder.
    for ix in supported {
        for a in &ix.account_defs {
            roles.entry(a.name.clone()).or_insert(Role::Placeholder);
        }
    }
    roles
}

/// True iff `name` will end up a Placeholder (referenced somewhere,
/// never fixed/signer/pda-defined).
fn will_be_placeholder(
    name: &str,
    supported: &[&Instruction],
    roles: &BTreeMap<String, Role>,
) -> bool {
    if roles.contains_key(name) {
        return false;
    }
    !supported.iter().any(|ix| {
        ix.account_defs
            .iter()
            .any(|a| a.name == name && a.pda.is_some())
    })
}

fn pda_has_arg_seed(pda: &PdaDef) -> bool {
    pda.seeds.iter().any(|s| matches!(s, SeedDef::Arg { .. }))
}

fn is_init_flavored(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("init") || n.starts_with("create") || n == "new"
}

/// Fixed bootstrap argument values (best-effort calls in setup).
fn bootstrap_args(ix: &Instruction, magnitude: u64) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for arg in &ix.arg_defs {
        let v = match arg.ty {
            IdlType::Bool => "false".to_string(),
            IdlType::U8 => format!("{}u8", magnitude.min(100)),
            IdlType::U16 => format!("{}u16", magnitude.min(50_000)),
            IdlType::U32 => format!("{}u32", magnitude.min(1_000_000)),
            IdlType::U64 => format!("{}u64", magnitude.min(1_000_000)),
            IdlType::U128 => format!("{}u128", magnitude.min(1_000_000)),
            // Setup scope: signer keypairs are `kp_*` locals.
            IdlType::Pubkey => format!(
                "{}.pubkey()",
                kp_field(first_signer(ix).expect("supported => pubkey args have a signer"))
            ),
            _ => "0".to_string(),
        };
        out.insert(arg.name.clone(), v);
    }
    out
}

/// First signer account on the instruction (fixture-held: pinned-address
/// signers are rejected by `instruction_supported`).
fn first_signer(ix: &Instruction) -> Option<&str> {
    ix.account_defs
        .iter()
        .find(|a| a.signer)
        .map(|a| a.name.as_str())
}

/// Bootstrap calls bind args as local `let`s so arg-seeded PDAs can
/// reference them by ident, mirroring the action-arm scope shape.
fn render_arg_lets(
    ix: &Instruction,
    values: &BTreeMap<String, String>,
    indent: &str,
) -> (String, BTreeMap<String, String>) {
    let mut lets = String::new();
    let mut exprs = BTreeMap::new();
    for arg in &ix.arg_defs {
        let ident = sanitize_ident(&arg.name);
        if let Some(v) = values.get(&arg.name) {
            lets.push_str(&format!("{indent}let {ident} = {v};\n"));
            exprs.insert(arg.name.clone(), ident);
        }
    }
    (lets, exprs)
}

// ---------------------------------------------------------------------------
// Type mapping.
// ---------------------------------------------------------------------------

/// Rust type for a fuzz-generatable scalar arg.
fn scalar_rust_type(ty: &IdlType) -> Option<&'static str> {
    match ty {
        IdlType::U8 => Some("u8"),
        IdlType::U16 => Some("u16"),
        IdlType::U32 => Some("u32"),
        IdlType::U64 => Some("u64"),
        IdlType::U128 => Some("u128"),
        IdlType::Bool => Some("bool"),
        _ => None,
    }
}

/// `#[range(..)]` for a fuzz arg, sized to the type.
fn range_attr(ty: &IdlType) -> Option<&'static str> {
    match ty {
        IdlType::U8 => Some("1..250"),
        IdlType::U16 => Some("1..50_000"),
        IdlType::U32 | IdlType::U64 | IdlType::U128 => Some("1..1_000_000"),
        _ => None,
    }
}

/// Per-arg-name range override — used for arguments whose semantics are
/// "upper slippage bound" / "lower slippage bound" rather than "amount".
/// Against snapshot-loaded state whose reserves may sit far above 1e6,
/// the deposit / swap slippage guards need much larger upper bounds to
/// have any chance of admitting a successful call (R3 §1.3). Names
/// covered are the industry-standard Solana AMM patterns.
fn range_attr_for_arg(arg_name: &str, ty: &IdlType) -> Option<String> {
    let lower = arg_name.to_ascii_lowercase();
    let is_upper_bound = lower.starts_with("max_")
        || lower.starts_with("maximum_")
        || lower.ends_with("_max")
        || lower.ends_with("_maximum")
        || lower.ends_with("_limit")
        || lower.contains("max_amount");
    let is_lower_bound = lower.starts_with("min_")
        || lower.starts_with("minimum_")
        || lower.ends_with("_min")
        || lower.ends_with("_minimum")
        || lower.contains("min_amount")
        || lower.contains("minimum_amount");
    match ty {
        IdlType::U32 | IdlType::U64 | IdlType::U128 if is_upper_bound => {
            // Loose upper slippage bound: allow the call to admit any
            // amount the pool math computes.
            Some("(u64::MAX / 2)..u64::MAX".to_string())
        }
        IdlType::U32 | IdlType::U64 | IdlType::U128 if is_lower_bound => {
            // Loose lower slippage bound: 1..2 accepts almost any output.
            Some("1..2".to_string())
        }
        _ => range_attr(ty).map(|s| s.to_string()),
    }
}

/// Rust type for a state-struct field.
fn rust_type(ty: &IdlType) -> Option<String> {
    Some(match ty {
        IdlType::U8 => "u8".into(),
        IdlType::U16 => "u16".into(),
        IdlType::U32 => "u32".into(),
        IdlType::U64 => "u64".into(),
        IdlType::U128 => "u128".into(),
        IdlType::I8 => "i8".into(),
        IdlType::I16 => "i16".into(),
        IdlType::I32 => "i32".into(),
        IdlType::I64 => "i64".into(),
        IdlType::I128 => "i128".into(),
        IdlType::Bool => "bool".into(),
        IdlType::F32 => "f32".into(),
        IdlType::F64 => "f64".into(),
        IdlType::Bytes => "Vec<u8>".into(),
        IdlType::String => "String".into(),
        IdlType::Pubkey => "Pubkey".into(),
        IdlType::Array(inner, len) => format!("[{}; {len}]", rust_type(inner)?),
        IdlType::Vec(inner) => format!("Vec<{}>", rust_type(inner)?),
        IdlType::Option(inner) => format!("Option<{}>", rust_type(inner)?),
        IdlType::Defined(name) => capitalize_snake(name),
        IdlType::Unknown(_) => return None,
    })
}

fn defined_deps(ty: &IdlType) -> Vec<String> {
    match ty {
        IdlType::Defined(name) => vec![name.clone()],
        IdlType::Array(inner, _) | IdlType::Vec(inner) | IdlType::Option(inner) => {
            defined_deps(inner)
        }
        _ => vec![],
    }
}

fn is_zero_copy(tdef: &TypeDef) -> bool {
    matches!(
        tdef.serialization.as_deref(),
        Some("bytemuck") | Some("bytemuckunsafe")
    )
}

/// Zero-copy layouts are only byte-compatible with sequential reading
/// when packed; dynamic types never appear in them.
fn check_layout_supported(tdef: &TypeDef) -> Result<(), EmitError> {
    if is_zero_copy(tdef) && !tdef.repr_packed {
        return Err(EmitError::Unsupported(format!(
            "zero-copy type `{}` uses a non-packed repr — its byte offsets include \
             alignment padding this emitter does not model",
            tdef.name
        )));
    }
    if is_zero_copy(tdef) {
        if let TypeDefBody::Struct { fields } = &tdef.body {
            for f in fields {
                if has_dynamic_type(&f.ty) {
                    return Err(EmitError::Unsupported(format!(
                        "zero-copy type `{}` carries dynamic field `{}` — inconsistent IDL",
                        tdef.name, f.name
                    )));
                }
            }
        }
    }
    Ok(())
}

fn has_dynamic_type(ty: &IdlType) -> bool {
    match ty {
        IdlType::Bytes | IdlType::String | IdlType::Vec(_) | IdlType::Option(_) => true,
        IdlType::Array(inner, _) => has_dynamic_type(inner),
        _ => false,
    }
}

fn enum_binding(name: &str, variants: &[String]) -> Result<String, EmitError> {
    let ident = capitalize_snake(name);
    let mut var_lines = String::new();
    let mut de_arms = String::new();
    for (i, v) in variants.iter().enumerate() {
        if v.starts_with("__data_variant_") || v.starts_with("__unknown_kind_") {
            return Err(EmitError::Unsupported(format!(
                "enum `{name}` has a data-carrying or unmodeled variant ({v}); only \
                 unit-variant enums are generated"
            )));
        }
        var_lines.push_str(&format!("    {v},\n"));
        de_arms.push_str(&format!("            {i} => Ok({ident}::{v}),\n"));
    }
    Ok(format!(
        "/// `{name}` enum, from the IDL `types` array (borsh u8 tag).\n\
         pub enum {ident} {{\n{var_lines}}}\n\
         impl anchor_lang::AnchorDeserialize for {ident} {{\n\
         \x20   fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {{\n\
         \x20       let tag: u8 = anchor_lang::AnchorDeserialize::deserialize_reader(reader)?;\n\
         \x20       match tag as usize {{\n{de_arms}\
         \x20           other => Err(std::io::Error::new(\n\
         \x20               std::io::ErrorKind::InvalidData,\n\
         \x20               format!(\"invalid enum tag {{other}} for {ident}\"),\n\
         \x20           )),\n\
         \x20       }}\n\
         \x20   }}\n\
         }}\n"
    ))
}

// ---------------------------------------------------------------------------
// Small shared helpers.
// ---------------------------------------------------------------------------

fn render_trident_stub(_surface: &ContractSurface, candidate: &InvariantCandidate) -> String {
    format!(
        "// Trident emit target is staged for Phase 1.\n\
         //\n\
         // Phase-0 ranking + candidate generation already runs end-to-end for\n\
         // `{}` (class = {}), but the Trident `#[init]` / `FuzzData` rendering\n\
         // shape is not yet wired. See docs/architecture.md §emit-targets.\n",
        candidate.name, candidate.class,
    )
}

fn disclosure_header(source: &InvariantSource) -> String {
    match source {
        InvariantSource::Manual => "Source: Manual.".into(),
        InvariantSource::Heuristic { suggester_version } => format!(
            "Source: Heuristic (suggester v{suggester_version}). \
             No AI suggestion in this candidate."
        ),
        InvariantSource::AiSuggested {
            model,
            prompt_version,
            timestamp_utc,
        } => format!(
            "Source: AI-SUGGESTED, UNVERIFIED until reviewed by author. \
             model={model} prompt_version={prompt_version} timestamp_utc={timestamp_utc}."
        ),
    }
}

fn byte_list(bytes: &[u8]) -> String {
    let mut out = String::from("[");
    for (i, b) in bytes.iter().enumerate() {
        if i == 0 {
            out.push_str(&format!("{b}u8"));
        } else {
            out.push_str(&format!(", {b}"));
        }
    }
    out.push(']');
    out
}

fn addr_field(name: &str) -> String {
    format!("a_{}", sanitize_ident(name))
}

fn kp_field(name: &str) -> String {
    format!("kp_{}", sanitize_ident(name))
}

/// IDL names → valid Rust snake_case idents (keyword-safe).
fn sanitize_ident(name: &str) -> String {
    let snake = to_snake_case(name);
    const KEYWORDS: &[&str] = &[
        "as", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false",
        "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
        "ref", "return", "self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while",
    ];
    if KEYWORDS.contains(&snake.as_str()) {
        format!("{snake}_")
    } else {
        snake
    }
}

fn to_snake_case(s: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for c in s.chars() {
        if c.is_ascii_uppercase() {
            if prev_lower {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
            prev_lower = false;
        } else {
            prev_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
            out.push(c);
        }
    }
    out
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// Convert snake_case → CamelCase for the generated `IxFoo` /
/// `IxFooAccounts` shape. `withdraw_admin` → `WithdrawAdmin`.
fn capitalize_snake(s: &str) -> String {
    s.split('_')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut chars = p.chars();
            match chars.next() {
                Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_invariants_anchor_core::EmitHints;
    use cf_invariants_anchor_suggest::ClassRegistry;

    const VAULT_IDL: &str =
        include_str!("../../../references/vault_ref/idls/vault_ref.json");
    const COUNTER_IDL: &str =
        include_str!("../../../references/counter_ref/idls/counter_ref.json");
    const ADMIN_IDL: &str =
        include_str!("../../../references/admin_ref/idls/admin_ref.json");
    const RAYDIUM_IDL: &str =
        include_str!("../tests/fixtures/raydium_cp_swap_onchain.json");

    fn surface(idl: &str) -> ContractSurface {
        cf_invariants_anchor_idl::ingest_bytes(idl.as_bytes()).expect("parse")
    }

    fn candidate_named(surface: &ContractSurface, name: &str) -> InvariantCandidate {
        ClassRegistry::default()
            .propose_all(surface)
            .into_iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("no candidate named {name}"))
    }

    // -- vault_ref (reference pair — CI compiles + runs this output) -----

    #[test]
    fn vault_balance_emit_is_idl_driven_and_standalone() {
        let s = surface(VAULT_IDL);
        let c = candidate_named(&s, "invariant_amount_conservation");
        let out = render(&s, &c, Target::Crucible).expect("render");
        // Real Crucible v0.2.0 macros.
        assert!(out.contains("#[fuzz_fixture]"));
        assert!(out.contains("#[invariant_test]"));
        assert!(out.contains("fuzz_assert_eq!"));
        assert!(out.contains("use crucible_fuzzer::"));
        // Standalone bindings: instruction discriminators from the IDL,
        // and NO dependency on the target crate.
        assert!(out.contains("&[242u8, 35, 198, 137, 82, 225, 242, 182]"));
        assert!(out.contains("pub struct IxDeposit"));
        assert!(out.contains("pub struct IxDepositAccounts"));
        assert!(!out.contains("use ::vault_ref"));
        assert!(!out.contains("instruction::Deposit"));
        // PDA seeds from the IDL, not hardcoded b"vault".
        assert!(out.contains("&[118u8, 97, 117, 108, 116][..]"));
        assert!(out.contains("kp_depositor"));
        // State struct generated from the IDL type layout.
        assert!(out.contains("pub struct Vault"));
        assert!(out.contains("read_anchor_account::<Vault>"));
        assert!(out.contains("&[211u8, 8, 232, 43, 2, 152, 117, 119]"));
        // Ledger arms + bookkeeping.
        assert!(out.contains("pub fn action_deposit"));
        assert!(out.contains("pub fn action_withdraw"));
        assert!(out.contains("expected_amount"));
        assert!(out.contains("saturating_add(amount as u128)"));
        assert!(out.contains("saturating_sub(amount as u128)"));
        assert!(out.contains("fn invariant_amount_conservation"));
        assert!(out.contains("Source: Heuristic"));
        assert!(!out.contains("crucible::invariants!"));
    }

    #[test]
    fn counter_monotonic_emit_uses_le_assertion_and_snapshot() {
        let s = surface(COUNTER_IDL);
        let c = candidate_named(&s, "invariant_lifetime_deposited_monotonic");
        let out = render(&s, &c, Target::Crucible).expect("render");
        assert!(out.contains("fuzz_assert_le!"));
        assert!(out.contains("last_seen_lifetime_deposited"));
        assert!(out.contains("Source: Heuristic"));
        assert!(!out.contains("expected_amount"));
    }

    #[test]
    fn admin_access_emit_uses_attacker_keypair_and_sticky_flag() {
        let s = surface(ADMIN_IDL);
        let c = candidate_named(&s, "invariant_withdraw_rejects_unauthorized");
        let out = render(&s, &c, Target::Crucible).expect("render");
        assert!(out.contains("let attacker = Keypair::new();"));
        assert!(out.contains("unauthorized_success_observed"));
        assert!(out.contains("pub fn action_attack_withdraw"));
        assert!(out.contains("fn invariant_withdraw_rejects_unauthorized"));
        // Seed deposit before attacks (positive-direction move).
        assert!(out.contains("Seed state via the positive-direction"));
    }

    // -- Raydium CP-Swap (real on-chain IDL — the R1 done condition) -----

    #[test]
    fn raydium_lp_supply_emit_renders_real_shapes() {
        let s = surface(RAYDIUM_IDL);
        let c = candidate_named(&s, "invariant_lp_supply_conservation");
        let out = render(&s, &c, Target::Crucible).expect("render");
        // Real argument shapes (C1 friction F6): deposit takes
        // lp_token_amount + maxima, not the vault template's `amount`.
        assert!(out.contains("pub lp_token_amount: u64"));
        assert!(out.contains("pub maximum_token_0_amount: u64"));
        assert!(out.contains("pub minimum_token_0_amount: u64"));
        // Real account structs (F5): 13-account deposit, not
        // {vault, depositor, system_program}.
        assert!(out.contains("pub token_0_vault: Pubkey"));
        assert!(out.contains("pub owner_lp_token: Pubkey"));
        assert!(!out.contains("vault: vault_pda"));
        // Real PDA seeds from the IDL: the vault_and_lp_mint_auth_seed
        // constant, not b"vault".
        assert!(out.contains(
            "&[118u8, 97, 117, 108, 116, 95, 97, 110, 100, 95, 108, 112, 95, 109, 105, 110, 116, 95, 97, 117, 116, 104, 95, 115, 101, 101, 100][..]"
        ));
        // Zero-copy PoolState generated from the IDL layout with its
        // real discriminator.
        assert!(out.contains("pub struct PoolState"));
        assert!(out.contains("&[247u8, 237, 227, 245, 215, 195, 222, 70]"));
        assert!(out.contains("repr(C, packed)"));
        assert!(out.contains("read_anchor_account::<PoolState>"));
        // Ledger binds the REAL arg (C1 §5: the old encoding added a
        // nonexistent `amount`).
        assert!(out.contains("saturating_add(lp_token_amount as u128)"));
        assert!(out.contains("saturating_sub(lp_token_amount as u128)"));
        // IDL-pinned fixed addresses come through.
        assert!(out.contains("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"));
        // The admin-keyed config instruction is excluded LOUDLY, not
        // faked: its signer is pinned to an address whose key we
        // cannot hold.
        assert!(out.contains("`create_amm_config`"));
        assert!(out.contains("pinned to address"));
        // Standalone: no dependency on the raydium crate under any name.
        assert!(!out.contains("use ::raydium"));
    }

    #[test]
    fn raydium_swap_arms_render_with_real_args() {
        let s = surface(RAYDIUM_IDL);
        let c = candidate_named(&s, "invariant_lp_supply_conservation");
        let out = render(&s, &c, Target::Crucible).expect("render");
        assert!(out.contains("pub fn action_swap_base_input"));
        assert!(out.contains("amount_in: u64"));
        assert!(out.contains("minimum_amount_out: u64"));
    }

    // -- pubkey args bind to the calling signer --------------------------

    #[test]
    fn pubkey_arg_binds_to_calling_signer_not_fuzz_param() {
        const KAMINO_IDL: &str =
            include_str!("../../../references/kamino_lending_ref/idls/kamino_lending_ref.json");
        let s = surface(KAMINO_IDL);
        let c = candidate_named(&s, "invariant_total_assets_conservation");
        let out = render(&s, &c, Target::Crucible).expect("render");
        // `set_admin(new_admin: pubkey)` renders as an action arm with
        // NO fuzz param for the pubkey — it is bound to the calling
        // signer's key.
        assert!(out.contains("pub fn action_set_admin(&mut self)"));
        assert!(out.contains("new_admin: self.kp_admin.pubkey()"));
        // In the attack probe the bound key is the attacker's.
        let c = candidate_named(&s, "invariant_set_admin_rejects_unauthorized");
        let out = render(&s, &c, Target::Crucible).expect("render");
        assert!(out.contains("new_admin: attacker.pubkey()"));
    }

    // -- loud failure modes (the kill-condition mechanics) ---------------

    #[test]
    fn unknown_class_is_a_loud_error() {
        let s = surface(VAULT_IDL);
        let mut c = candidate_named(&s, "invariant_amount_conservation");
        c.class = "totally_made_up".into();
        match render(&s, &c, Target::Crucible) {
            Err(EmitError::UnknownClass(cl)) => assert_eq!(cl, "totally_made_up"),
            other => panic!("expected UnknownClass, got {other:?}"),
        }
    }

    #[test]
    fn unbindable_account_type_is_missing_idl_data() {
        let s = surface(VAULT_IDL);
        let mut c = candidate_named(&s, "invariant_amount_conservation");
        c.emit_hints = EmitHints {
            account_type: "Nonexistent".into(),
            field: "amount".into(),
            ..c.emit_hints
        };
        match render(&s, &c, Target::Crucible) {
            Err(EmitError::MissingIdlData(msg)) => {
                assert!(msg.contains("Nonexistent"), "{msg}");
            }
            other => panic!("expected MissingIdlData, got {other:?}"),
        }
    }

    #[test]
    fn legacy_idl_without_discriminators_is_rejected_per_instruction() {
        let mut s = surface(VAULT_IDL);
        for ix in &mut s.instructions {
            ix.discriminator.clear();
        }
        let c = candidate_named(&surface(VAULT_IDL), "invariant_amount_conservation");
        match render(&s, &c, Target::Crucible) {
            Err(EmitError::Unsupported(msg)) => {
                assert!(msg.contains("discriminator"), "{msg}");
            }
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }

    #[test]
    fn trident_emit_is_phase1_stub() {
        let s = surface(VAULT_IDL);
        let c = candidate_named(&s, "invariant_amount_conservation");
        let out = render(&s, &c, Target::Trident).expect("stub");
        assert!(out.contains("Trident emit target is staged for Phase 1"));
    }

    #[test]
    fn ai_source_renders_ai_disclosure_banner() {
        let s = surface(VAULT_IDL);
        let mut c = candidate_named(&s, "invariant_amount_conservation");
        c.source = InvariantSource::AiSuggested {
            model: "claude-sonnet-4-6".into(),
            prompt_version: "invariant_suggestion_v1".into(),
            timestamp_utc: "2026-06-01T18:00:00Z".into(),
        };
        let out = render(&s, &c, Target::Crucible).expect("render");
        assert!(out.contains("Source: AI-SUGGESTED, UNVERIFIED"));
        assert!(out.contains("claude-sonnet-4-6"));
    }
}
