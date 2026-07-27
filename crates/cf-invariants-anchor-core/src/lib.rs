// cf-invariants-anchor-core — shared types.
//
// `ContractSurface` is the canonical mid-form between Anchor IDL ingest
// and the invariant suggester. `InvariantCandidate` carries a
// `source` field that distinguishes manual / AI-suggested / heuristic
// candidates — the renderer keys disclosure on this.

use serde::{Deserialize, Serialize};

/// Token-like field on a stored account that participates in
/// balance-conservation reasoning. The Phase-0 suggester ranks these.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BalanceField {
    /// Account type that owns the field (e.g. `Vault`).
    pub account: String,
    /// Field name on the account (e.g. `amount`).
    pub field: String,
    /// Solidity-style scalar type (`u64`, `u128`, `i64`, ...).
    pub ty: String,
}

/// Scalar/composite type as the Anchor 0.30+ IDL spells it. The emit
/// crate maps these to Rust types when it generates standalone client
/// bindings, so every variant here must round-trip losslessly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IdlType {
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    Bool,
    F32,
    F64,
    Bytes,
    String,
    Pubkey,
    Array(Box<IdlType>, usize),
    Vec(Box<IdlType>),
    Option(Box<IdlType>),
    Defined(String),
    /// A type shape this parser does not model. Carried verbatim so
    /// emit can reject it LOUDLY instead of silently mis-rendering.
    Unknown(String),
}

/// One typed instruction argument (name + IDL type).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct IxArgDef {
    pub name: String,
    #[serde(default = "IdlType::unknown_default")]
    pub ty: IdlType,
}

impl IdlType {
    fn unknown_default() -> IdlType {
        IdlType::Unknown("absent".into())
    }
}

impl Default for IdlType {
    fn default() -> Self {
        IdlType::Unknown("absent".into())
    }
}

/// One PDA seed as the Anchor 0.30+ IDL carries it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SeedDef {
    /// Literal bytes.
    Const { value: Vec<u8> },
    /// Pubkey of another account in the same instruction (`path` may
    /// be dotted for account-data field refs — emit rejects those).
    Account { path: String },
    /// Serialized bytes of an instruction argument.
    Arg { path: String },
}

/// PDA derivation spec on an instruction account.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PdaDef {
    pub seeds: Vec<SeedDef>,
    /// Deriving program when not the target program (e.g. the ATA
    /// program). `Const` bytes are a 32-byte pubkey.
    #[serde(default)]
    pub program: Option<SeedDef>,
}

/// Full metadata for one account slot on one instruction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct IxAccountDef {
    pub name: String,
    #[serde(default)]
    pub writable: bool,
    #[serde(default)]
    pub signer: bool,
    #[serde(default)]
    pub optional: bool,
    /// Fixed address (base58) if the IDL pins one.
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub pda: Option<PdaDef>,
}

/// One Anchor program instruction (one entry in the IDL `instructions`
/// array).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Instruction {
    pub name: String,
    /// Argument names, ordered.
    pub args: Vec<String>,
    /// Account names referenced (writable + read-only).
    pub accounts: Vec<String>,
    /// 8-byte Anchor instruction discriminator (empty for legacy IDLs).
    #[serde(default)]
    pub discriminator: Vec<u8>,
    /// Typed argument list (parallel to `args`; carries IDL types).
    #[serde(default)]
    pub arg_defs: Vec<IxArgDef>,
    /// Full account metadata (parallel to `accounts`).
    #[serde(default)]
    pub account_defs: Vec<IxAccountDef>,
}

/// One state account type the program owns (IDL `accounts` array
/// entry): the name links into `type_defs`; the discriminator is what
/// `read_anchor_account` verifies at runtime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AccountTypeDef {
    pub name: String,
    #[serde(default)]
    pub discriminator: Vec<u8>,
}

/// Field on a defined type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldDef {
    pub name: String,
    pub ty: IdlType,
}

/// Body of a defined type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TypeDefBody {
    Struct { fields: Vec<FieldDef> },
    /// Unit variants only — data-carrying variants come through as
    /// `Unknown` fields on the variant list and emit rejects them.
    Enum { variants: Vec<String> },
}

/// One entry in the IDL `types` array.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TypeDef {
    pub name: String,
    pub body: TypeDefBody,
    /// IDL `serialization` marker (`bytemuck` / `bytemuckunsafe` for
    /// zero-copy accounts; absent = borsh).
    #[serde(default)]
    pub serialization: Option<String>,
    /// True iff the IDL `repr` says `packed` — the only zero-copy
    /// layout whose byte offsets match sequential borsh reading.
    #[serde(default)]
    pub repr_packed: bool,
}

/// Canonical mid-form: what cf-invariants-anchor-idl produces and what
/// cf-invariants-anchor-suggest consumes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ContractSurface {
    pub program_id: String,
    pub program_name: String,
    pub instructions: Vec<Instruction>,
    /// All scalar balance-bearing fields across all account types.
    pub balance_fields: Vec<BalanceField>,
    /// State account types (name + discriminator) from the IDL
    /// `accounts` array.
    #[serde(default)]
    pub account_types: Vec<AccountTypeDef>,
    /// Full defined-type layouts from the IDL `types` array.
    #[serde(default)]
    pub type_defs: Vec<TypeDef>,
}

impl ContractSurface {
    /// Instructions that name-match the conservation movement pattern
    /// (deposit / withdraw / mint / burn / transfer / claim).
    ///
    /// Phase-0 heuristic — deliberately conservative. The Phase-1 path
    /// is structural (read the IR / account-mut-set), not lexical.
    pub fn movement_instructions(&self) -> Vec<&Instruction> {
        const MARKERS: &[&str] = &[
            "deposit", "withdraw", "mint", "burn", "transfer", "claim",
            "redeem", "stake", "unstake",
        ];
        self.instructions
            .iter()
            .filter(|ix| {
                let n = ix.name.to_ascii_lowercase();
                MARKERS.iter().any(|m| n.contains(m))
            })
            .collect()
    }
}

/// Provenance for an invariant. The scorecard renderer reads this to
/// decide whether to emit the AI-disclosure banner.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind")]
pub enum InvariantSource {
    /// Hand-authored by the user.
    Manual,
    /// Heuristic suggester (no AI call). Used in offline mode and tests.
    Heuristic {
        suggester_version: String,
    },
    /// AI-suggested. Banner-required on the scorecard.
    AiSuggested {
        model: String,
        prompt_version: String,
        timestamp_utc: String,
    },
}

impl InvariantSource {
    pub fn is_ai_suggested(&self) -> bool {
        matches!(self, InvariantSource::AiSuggested { .. })
    }
}

/// One candidate invariant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvariantCandidate {
    /// Stable identifier — used as the `#[invariant_test]` function name
    /// when emitted.
    pub name: String,
    /// One-line summary.
    pub summary: String,
    /// Class — `balance_conservation` in Phase 0; the ranking framework
    /// is extensible (see suggester ClassRegistry).
    pub class: String,
    /// Phase-0 ranking: higher = stronger candidate. Bounded \[0.0, 1.0\].
    pub rank: f32,
    /// One-paragraph rationale shown to the developer before they accept.
    pub rationale: String,
    /// The Crucible-side assertion plus the fixture-side bookkeeping
    /// required to evaluate it.
    pub emit_hints: EmitHints,
    pub source: InvariantSource,
}

/// One fixture-side ledger move: on successful `action`, apply `arg`
/// (an instruction argument name) to the tracked field with the given
/// direction. The SUGGESTER decides these (it owns the hypothesis);
/// the emit crate renders them verbatim.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LedgerMove {
    pub action: String,
    pub arg: String,
    /// `true` = saturating_add, `false` = saturating_sub.
    pub add: bool,
}

/// Fields the emit crate needs to synthesise a Crucible `#[invariant_test]`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct EmitHints {
    /// Account type to read with `ctx.read_anchor_account::<T>(&pda)`.
    pub account_type: String,
    /// Field on the account to assert against (e.g. `amount`).
    pub field: String,
    /// Expression to compute the expected value at fixture-side. The
    /// emit crate substitutes this verbatim into the rendered fixture.
    /// For Phase-0 balance-conservation: `fixture.expected_amount`.
    pub expected_expression: String,
    /// Action names whose post-conditions update the expected expression.
    /// The emit crate generates an `action_*` arm per name.
    pub action_names: Vec<String>,
    /// Ledger bookkeeping per action (balance_conservation). Also used
    /// by access_control setup to seed positive-direction state.
    #[serde(default)]
    pub ledger_moves: Vec<LedgerMove>,
    /// Relation-class bundle: one candidate carries N specs, each
    /// rendered as an independent `#[invariant_test]` fn in the same
    /// fixture. R2 (relation_invariants class) uses this to emit multi-
    /// field / token-account bindings; other classes leave it empty.
    #[serde(default)]
    pub relation_specs: Vec<RelationSpec>,
    /// Suppressed relation specs — helper-fn-name prefixes the emitter
    /// should NOT render (e.g. `helper_pool_state_matches_lp_mint_supply`
    /// to suppress the CP-Swap MINIMUM_LIQUIDITY-lockup RC-D finding once
    /// the fixture author has validated it as by-design). Wired R3 to
    /// close the `ok > 0` gap when a validated-by-design finding would
    /// otherwise fail every seed and starve the corpus.
    #[serde(default)]
    pub suppressed_specs: Vec<String>,
}

/// One relation-invariant hypothesis. Emitter dispatches on `kind` and
/// pulls the fields it needs; each kind is a distinct assertion shape.
///
/// Every field carries fixture-visible symbol names (instruction-account
/// names as they appear in the IDL); the emitter maps those to the
/// `a_<name>` / `kp_<name>` locals it already generates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RelationSpec {
    /// RC-A: cross-account name binding. `state_account_type` is the
    /// program-owned struct read at `state_account_name`; `pubkey_field`
    /// is the pubkey field on that struct; `bound_account_name` is the
    /// sibling instruction account whose address the field must equal.
    Binding {
        state_account_type: String,
        state_account_name: String,
        pubkey_field: String,
        bound_account_name: String,
    },
    /// RC-D: cross-program supply conservation. `state_account_type` /
    /// `state_account_name` / `supply_field` name a `u64` on the pool;
    /// `mint_account_name` is the sibling SPL Mint account whose
    /// `Mint::supply` the field must equal.
    SupplyMint {
        state_account_type: String,
        state_account_name: String,
        supply_field: String,
        mint_account_name: String,
    },
    /// RC-E: fee-accumulator monotonicity. The named `u64` field on
    /// the state account must never decrease between successive
    /// observations. Fixture caches `prev_<field>` between actions.
    FeeMonotone {
        state_account_type: String,
        state_account_name: String,
        field: String,
    },
    /// RC-G: vault ownership + mint binding. The named SPL token
    /// account's `mint` and `owner` must equal the fixture-visible
    /// expected values (`expected_mint_name` = a sibling mint account;
    /// `expected_owner_name` = the pool authority PDA).
    VaultBinding {
        vault_account_name: String,
        expected_mint_name: String,
        expected_owner_name: String,
    },
    /// RC-I: mint-authority binding. The named SPL mint's
    /// `mint_authority` must equal the fixture-visible authority.
    MintAuthority {
        mint_account_name: String,
        expected_authority_name: String,
    },
}

/// Scorecard envelope — mirrors cf-invariants (Cairo) shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scorecard {
    pub invariants_total: usize,
    pub invariants_violated: usize,
    pub ai_suggestions_included: usize,
    pub runtime_ms: u64,
    pub crucible_version: String,
    pub counterexamples: Vec<Counterexample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Counterexample {
    pub invariant_name: String,
    pub seed: String,
    pub failing_sequence: String,
    pub witness: serde_json::Value,
}
