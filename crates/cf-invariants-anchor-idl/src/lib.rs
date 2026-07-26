// cf-invariants-anchor-idl — Anchor IDL → ContractSurface.
//
// Targets the Anchor 0.30+ IDL shape (the same one Codama emits and
// Crucible's `declare_fuzz_program!` consumes). We intentionally do
// NOT depend on `anchor-lang-idl` — it would drag the full Anchor +
// solana-program closure for one struct parse.
//
// R1 (emit-codegen rewrite) widened this parser from "the fields the
// suggester ranks" to the full codegen surface: instruction
// discriminators, typed args, per-account metadata (writable / signer /
// optional / fixed address / PDA seed spec), state-account
// discriminators, and complete defined-type layouts including the
// zero-copy serialization/repr markers. Everything the emit crate
// renders comes from here; nothing is assumed from the reference
// examples.

use cf_invariants_anchor_core::{
    AccountTypeDef, BalanceField, ContractSurface, FieldDef, IdlType, Instruction, IxAccountDef,
    IxArgDef, PdaDef, SeedDef, TypeDef, TypeDefBody,
};
use serde::Deserialize;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IdlError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("idl missing required field: {0}")]
    MissingField(&'static str),
}

#[derive(Debug, Deserialize)]
struct RawIdl {
    address: Option<String>,
    metadata: Option<RawMetadata>,
    name: Option<String>,
    instructions: Vec<RawInstruction>,
    #[serde(default)]
    accounts: Vec<RawAccount>,
    #[serde(default)]
    types: Vec<RawTypeDef>,
}

#[derive(Debug, Deserialize)]
struct RawMetadata {
    name: Option<String>,
    #[allow(dead_code)]
    version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawInstruction {
    name: String,
    #[serde(default)]
    discriminator: Vec<u8>,
    #[serde(default)]
    args: Vec<RawArg>,
    #[serde(default)]
    accounts: Vec<RawIxAccount>,
}

#[derive(Debug, Deserialize)]
struct RawArg {
    name: String,
    #[serde(rename = "type", default)]
    ty: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct RawIxAccount {
    name: String,
    #[serde(default)]
    writable: bool,
    #[serde(default)]
    signer: bool,
    #[serde(default)]
    optional: bool,
    #[serde(default)]
    address: Option<String>,
    #[serde(default)]
    pda: Option<RawPda>,
}

#[derive(Debug, Deserialize)]
struct RawPda {
    #[serde(default)]
    seeds: Vec<serde_json::Value>,
    #[serde(default)]
    program: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct RawAccount {
    name: String,
    #[serde(default)]
    discriminator: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct RawTypeDef {
    name: String,
    #[serde(rename = "type")]
    ty: RawTypeBody,
    #[serde(default)]
    serialization: Option<String>,
    #[serde(default)]
    repr: Option<RawRepr>,
}

#[derive(Debug, Deserialize)]
struct RawRepr {
    #[allow(dead_code)]
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    packed: bool,
}

#[derive(Debug, Deserialize)]
struct RawTypeBody {
    kind: String,
    #[serde(default)]
    fields: Vec<RawField>,
    #[serde(default)]
    variants: Vec<RawVariant>,
}

#[derive(Debug, Deserialize)]
struct RawField {
    name: String,
    #[serde(rename = "type")]
    ty: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct RawVariant {
    name: String,
    #[serde(default)]
    fields: Vec<serde_json::Value>,
}

/// Parse an Anchor IDL JSON file into a ContractSurface.
pub fn ingest_path(path: &Path) -> Result<ContractSurface, IdlError> {
    let bytes = std::fs::read(path)?;
    ingest_bytes(&bytes)
}

/// Parse Anchor IDL JSON bytes into a ContractSurface.
pub fn ingest_bytes(bytes: &[u8]) -> Result<ContractSurface, IdlError> {
    let raw: RawIdl = serde_json::from_slice(bytes)?;

    let program_id = raw
        .address
        .clone()
        .ok_or(IdlError::MissingField("address"))?;
    let program_name = raw
        .metadata
        .as_ref()
        .and_then(|m| m.name.clone())
        .or_else(|| raw.name.clone())
        .ok_or(IdlError::MissingField("metadata.name | name"))?;

    let instructions = raw
        .instructions
        .into_iter()
        .map(|ix| Instruction {
            name: ix.name,
            args: ix.args.iter().map(|a| a.name.clone()).collect(),
            accounts: ix.accounts.iter().map(|a| a.name.clone()).collect(),
            discriminator: ix.discriminator,
            arg_defs: ix
                .args
                .into_iter()
                .map(|a| IxArgDef {
                    name: a.name,
                    ty: parse_idl_type(&a.ty),
                })
                .collect(),
            account_defs: ix
                .accounts
                .into_iter()
                .map(|a| IxAccountDef {
                    name: a.name,
                    writable: a.writable,
                    signer: a.signer,
                    optional: a.optional,
                    address: a.address,
                    pda: a.pda.map(parse_pda),
                })
                .collect(),
        })
        .collect::<Vec<_>>();

    let account_types: Vec<AccountTypeDef> = raw
        .accounts
        .iter()
        .map(|a| AccountTypeDef {
            name: a.name.clone(),
            discriminator: a.discriminator.clone(),
        })
        .collect();

    let type_defs: Vec<TypeDef> = raw.types.iter().map(parse_type_def).collect();

    // Balance-bearing fields = scalar uint fields on struct accounts
    // referenced by the `accounts` array. Phase 0 heuristic; refined
    // by structural reasoning in Phase 1 (account-mutability set across
    // movement instructions).
    let account_names: Vec<&str> = raw.accounts.iter().map(|a| a.name.as_str()).collect();
    let mut balance_fields = Vec::new();
    for tdef in &raw.types {
        if tdef.ty.kind != "struct" {
            continue;
        }
        if !account_names.contains(&tdef.name.as_str()) {
            continue;
        }
        for f in &tdef.ty.fields {
            if let Some(ty_str) = scalar_uint_type(&f.ty) {
                balance_fields.push(BalanceField {
                    account: tdef.name.clone(),
                    field: f.name.clone(),
                    ty: ty_str.to_string(),
                });
            }
        }
    }

    Ok(ContractSurface {
        program_id,
        program_name,
        instructions,
        balance_fields,
        account_types,
        type_defs,
    })
}

fn parse_pda(raw: RawPda) -> PdaDef {
    PdaDef {
        seeds: raw.seeds.iter().map(parse_seed).collect(),
        program: raw.program.as_ref().map(parse_seed),
    }
}

fn parse_seed(v: &serde_json::Value) -> SeedDef {
    let kind = v.get("kind").and_then(|k| k.as_str()).unwrap_or("");
    match kind {
        "const" => SeedDef::Const {
            value: v
                .get("value")
                .and_then(|x| x.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|b| b.as_u64().map(|b| b as u8))
                        .collect()
                })
                .unwrap_or_default(),
        },
        "account" => SeedDef::Account {
            path: v
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or("")
                .to_string(),
        },
        "arg" => SeedDef::Arg {
            path: v
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or("")
                .to_string(),
        },
        other => SeedDef::Account {
            // Unknown seed kind: carry a path that can never resolve so
            // the emit crate fails LOUDLY naming the kind, instead of
            // silently deriving a wrong PDA.
            path: format!("__unknown_seed_kind_{other}"),
        },
    }
}

fn parse_type_def(raw: &RawTypeDef) -> TypeDef {
    let body = match raw.ty.kind.as_str() {
        "struct" => TypeDefBody::Struct {
            fields: raw
                .ty
                .fields
                .iter()
                .map(|f| FieldDef {
                    name: f.name.clone(),
                    ty: parse_idl_type(&f.ty),
                })
                .collect(),
        },
        "enum" => TypeDefBody::Enum {
            variants: raw
                .ty
                .variants
                .iter()
                .map(|v| {
                    if v.fields.is_empty() {
                        v.name.clone()
                    } else {
                        // Data-carrying variant — mark so emit rejects.
                        format!("__data_variant_{}", v.name)
                    }
                })
                .collect(),
        },
        other => TypeDefBody::Enum {
            variants: vec![format!("__unknown_kind_{other}")],
        },
    };
    TypeDef {
        name: raw.name.clone(),
        body,
        serialization: raw.serialization.clone(),
        repr_packed: raw.repr.as_ref().map(|r| r.packed).unwrap_or(false),
    }
}

/// Map the IDL's JSON type spelling into the typed `IdlType` mid-form.
fn parse_idl_type(ty: &serde_json::Value) -> IdlType {
    if let Some(s) = ty.as_str() {
        return match s {
            "u8" => IdlType::U8,
            "u16" => IdlType::U16,
            "u32" => IdlType::U32,
            "u64" => IdlType::U64,
            "u128" => IdlType::U128,
            "i8" => IdlType::I8,
            "i16" => IdlType::I16,
            "i32" => IdlType::I32,
            "i64" => IdlType::I64,
            "i128" => IdlType::I128,
            "bool" => IdlType::Bool,
            "f32" => IdlType::F32,
            "f64" => IdlType::F64,
            "bytes" => IdlType::Bytes,
            "string" => IdlType::String,
            // Both spellings exist in the wild.
            "pubkey" | "publicKey" => IdlType::Pubkey,
            other => IdlType::Unknown(other.to_string()),
        };
    }
    if let Some(obj) = ty.as_object() {
        if let Some(arr) = obj.get("array").and_then(|a| a.as_array()) {
            if arr.len() == 2 {
                if let Some(len) = arr[1].as_u64() {
                    return IdlType::Array(Box::new(parse_idl_type(&arr[0])), len as usize);
                }
            }
        }
        if let Some(inner) = obj.get("vec") {
            return IdlType::Vec(Box::new(parse_idl_type(inner)));
        }
        if let Some(inner) = obj.get("option") {
            return IdlType::Option(Box::new(parse_idl_type(inner)));
        }
        if let Some(defined) = obj.get("defined") {
            // 0.30+ shape: {"defined": {"name": "X"}}; older: {"defined": "X"}.
            if let Some(name) = defined.as_str() {
                return IdlType::Defined(name.to_string());
            }
            if let Some(name) = defined.get("name").and_then(|n| n.as_str()) {
                return IdlType::Defined(name.to_string());
            }
        }
    }
    IdlType::Unknown(ty.to_string())
}

/// Return the IDL type string iff this field is a scalar unsigned integer.
fn scalar_uint_type(ty: &serde_json::Value) -> Option<&'static str> {
    let s = ty.as_str()?;
    match s {
        "u8" => Some("u8"),
        "u16" => Some("u16"),
        "u32" => Some("u32"),
        "u64" => Some("u64"),
        "u128" => Some("u128"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../../references/vault_ref/idls/vault_ref.json");

    #[test]
    fn ingests_vault_ref_surface() {
        let surface = ingest_bytes(FIXTURE.as_bytes()).expect("parse");
        assert_eq!(surface.program_name, "vault_ref");
        assert_eq!(
            surface.program_id,
            "Va111tRef1111111111111111111111111111111111"
        );
        let names: Vec<_> = surface.instructions.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"deposit") && names.contains(&"withdraw"));
        assert!(surface
            .balance_fields
            .iter()
            .any(|f| f.account == "Vault" && f.field == "amount" && f.ty == "u64"));
    }

    #[test]
    fn movement_instructions_pick_up_deposit_withdraw() {
        let surface = ingest_bytes(FIXTURE.as_bytes()).unwrap();
        let movement: Vec<_> = surface
            .movement_instructions()
            .into_iter()
            .map(|i| i.name.as_str())
            .collect();
        assert!(movement.contains(&"deposit"));
        assert!(movement.contains(&"withdraw"));
        assert!(!movement.contains(&"initialize"));
    }

    #[test]
    fn ingests_full_codegen_surface() {
        let surface = ingest_bytes(FIXTURE.as_bytes()).unwrap();
        let deposit = surface
            .instructions
            .iter()
            .find(|i| i.name == "deposit")
            .unwrap();
        // Discriminator carried verbatim.
        assert_eq!(
            deposit.discriminator,
            vec![242, 35, 198, 137, 82, 225, 242, 182]
        );
        // Typed args.
        assert_eq!(deposit.arg_defs.len(), 1);
        assert_eq!(deposit.arg_defs[0].name, "amount");
        assert_eq!(deposit.arg_defs[0].ty, IdlType::U64);
        // Account metadata: vault is a PDA with [b"vault", depositor].
        let vault = deposit
            .account_defs
            .iter()
            .find(|a| a.name == "vault")
            .unwrap();
        assert!(vault.writable && !vault.signer);
        let pda = vault.pda.as_ref().unwrap();
        assert_eq!(
            pda.seeds[0],
            SeedDef::Const {
                value: b"vault".to_vec()
            }
        );
        assert_eq!(
            pda.seeds[1],
            SeedDef::Account {
                path: "depositor".into()
            }
        );
        // depositor is a signer; system_program has a fixed address.
        let depositor = deposit
            .account_defs
            .iter()
            .find(|a| a.name == "depositor")
            .unwrap();
        assert!(depositor.signer);
        let sys = deposit
            .account_defs
            .iter()
            .find(|a| a.name == "system_program")
            .unwrap();
        assert_eq!(
            sys.address.as_deref(),
            Some("11111111111111111111111111111111")
        );
        // State account discriminator + type layout.
        let vault_ty = surface
            .account_types
            .iter()
            .find(|a| a.name == "Vault")
            .unwrap();
        assert_eq!(vault_ty.discriminator.len(), 8);
        let vault_def = surface.type_defs.iter().find(|t| t.name == "Vault").unwrap();
        match &vault_def.body {
            TypeDefBody::Struct { fields } => {
                assert_eq!(fields[0].name, "depositor");
                assert_eq!(fields[0].ty, IdlType::Pubkey);
                assert_eq!(fields[1].name, "amount");
                assert_eq!(fields[1].ty, IdlType::U64);
            }
            other => panic!("expected struct, got {other:?}"),
        }
    }

    #[test]
    fn parses_zero_copy_markers_and_composite_types() {
        let idl = r#"{
            "address": "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C",
            "metadata": {"name": "zc_ref", "version": "0.1.0"},
            "instructions": [
                {"name": "poke", "discriminator": [1,2,3,4,5,6,7,8],
                 "args": [{"name": "index", "type": "u16"}],
                 "accounts": [
                   {"name": "cfg", "writable": true,
                    "pda": {"seeds": [
                      {"kind": "const", "value": [99, 102, 103]},
                      {"kind": "arg", "path": "index"}]}}
                 ]}
            ],
            "accounts": [{"name": "Pool", "discriminator": [9,9,9,9,9,9,9,9]}],
            "types": [
                {"name": "Pool",
                 "serialization": "bytemuckunsafe",
                 "repr": {"kind": "c", "packed": true},
                 "type": {"kind": "struct", "fields": [
                    {"name": "mint", "type": "pubkey"},
                    {"name": "obs", "type": {"array": [{"defined": {"name": "Obs"}}, 4]}},
                    {"name": "pad", "type": {"array": ["u64", 8]}}
                 ]}},
                {"name": "Obs", "type": {"kind": "struct", "fields": [
                    {"name": "ts", "type": "u64"}]}},
                {"name": "Mode", "type": {"kind": "enum", "variants": [
                    {"name": "A"}, {"name": "B"}]}}
            ]
        }"#;
        let surface = ingest_bytes(idl.as_bytes()).unwrap();
        let pool = surface.type_defs.iter().find(|t| t.name == "Pool").unwrap();
        assert_eq!(pool.serialization.as_deref(), Some("bytemuckunsafe"));
        assert!(pool.repr_packed);
        match &pool.body {
            TypeDefBody::Struct { fields } => {
                assert_eq!(
                    fields[1].ty,
                    IdlType::Array(Box::new(IdlType::Defined("Obs".into())), 4)
                );
                assert_eq!(fields[2].ty, IdlType::Array(Box::new(IdlType::U64), 8));
            }
            other => panic!("expected struct, got {other:?}"),
        }
        let mode = surface.type_defs.iter().find(|t| t.name == "Mode").unwrap();
        assert_eq!(
            mode.body,
            TypeDefBody::Enum {
                variants: vec!["A".into(), "B".into()]
            }
        );
        // Arg-kind PDA seed round-trips.
        let poke = &surface.instructions[0];
        let cfg = &poke.account_defs[0];
        assert_eq!(
            cfg.pda.as_ref().unwrap().seeds[1],
            SeedDef::Arg {
                path: "index".into()
            }
        );
    }
}
