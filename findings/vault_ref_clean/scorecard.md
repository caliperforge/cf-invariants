# cf-invariants-anchor scorecard (CI capture, clean variant)

Captured by `.github/workflows/ci.yml` on 2026-06-02T14:04:01Z.
Toolchain: rustc 1.96.0, solana 2.1.21, platform-tools v1.52, crucible v0.2.0 (anchor-lang 1.0.1).
Crucible timeout: 30s.

## Summary

- Invariants total: **1**
- Invariants violated: **0**
- AI-suggested invariants in this run: **0**
- Crucible version: `0.2.0`

## Raw output

```
    Updating crates.io index
     Locking 510 packages to latest compatible versions
      Adding generic-array v0.14.7 (available: v0.14.9)
      Adding solana-address-lookup-table-interface v3.0.1 (available: v3.1.0)
      Adding solana-clock v3.0.1 (available: v3.1.0)
      Adding solana-epoch-schedule v3.0.0 (available: v3.1.0)
      Adding solana-instruction v3.1.0 (available: v3.4.0)
      Adding solana-instructions-sysvar v3.0.0 (available: v3.0.1)
      Adding solana-keypair v3.1.0 (available: v3.1.2)
      Adding solana-loader-v3-interface v6.1.0 (available: v6.1.1)
      Adding solana-message v3.0.1 (available: v3.1.0)
      Adding solana-nonce v3.0.0 (available: v3.2.0)
      Adding solana-pubkey v3.0.0 (available: v4.2.0)
      Adding solana-signature v3.1.0 (available: v3.4.0)
      Adding solana-stable-layout v3.0.0 (available: v3.0.1)
      Adding solana-system-interface v3.0.0 (available: v3.2.0)
      Adding solana-transaction v3.0.2 (available: v3.1.0)
      Adding solana-transaction-error v3.0.0 (available: v3.2.0)
      Adding spl-token-group-interface v0.7.1 (available: v0.7.2)
 Downloading crates ...
  Downloaded aead v0.5.2
  Downloaded adler2 v2.0.1
  Downloaded fnv v1.0.7
  Downloaded addr2line v0.25.1
  Downloaded addr2line v0.24.2
  Downloaded cobs v0.3.0
  Downloaded base16ct v0.2.0
  Downloaded inout v0.1.4
  Downloaded dtor v0.1.1
  Downloaded konst_macro_rules v0.2.19
  Downloaded foldhash v0.1.5
  Downloaded embedded-io v0.4.0
  Downloaded find-msvc-tools v0.1.9
  Downloaded percent-encoding v2.3.2
  Downloaded simd-adler32 v0.3.9
  Downloaded cfg_eval v0.1.2
  Downloaded enum-ordinalize v4.3.2
  Downloaded fallible-iterator v0.3.0
  Downloaded enum-iterator v1.5.0
  Downloaded solana-blake3-hasher v3.1.0
  Downloaded solana-account v3.4.0
  Downloaded solana-bincode v3.1.0
  Downloaded solana-fee v3.1.14
  Downloaded solana-derivation-path v3.0.0
  Downloaded solana-signer v3.0.0
  Downloaded solana-svm-callback v3.1.14
  Downloaded solana-transaction-error v3.0.0
  Downloaded solana-system-interface v3.0.0
  Downloaded static_assertions v1.1.0
  Downloaded universal-hash v0.5.1
  Downloaded solana-transaction-context v3.1.14
  Downloaded void v1.0.2
  Downloaded wait-timeout v0.2.1
  Downloaded terminal_size v0.4.4
  Downloaded typed-builder-macro v0.22.0
  Downloaded spl-pod v0.7.3
  Downloaded uds v0.4.2
  Downloaded xxhash-rust v0.8.15
  Downloaded virtue v0.0.18
  Downloaded unty v0.0.4
  Downloaded unicode-xid v0.2.6
  Downloaded tuple_list v0.1.3
  Downloaded spl-token-confidential-transfer-proof-generation v0.5.1
  Downloaded typewit v1.15.2
  Downloaded gimli v0.32.3
  Downloaded zerocopy v0.8.50
  Downloaded keccak v0.1.6
  Downloaded solana-message v3.0.1
  Downloaded postcard v1.1.3
  Downloaded zeroize_derive v1.4.3
  Downloaded regex v1.12.3
  Downloaded solana-compute-budget-program v3.1.14
  Downloaded libmimalloc-sys v0.1.49
  Downloaded itertools v0.12.1
  Downloaded uuid v1.23.2
  Downloaded light-poseidon v0.2.0
  Downloaded light-poseidon v0.4.0
  Downloaded libafl v0.15.4
  Downloaded hashbrown v0.16.1
  Downloaded nix v0.30.1
  Downloaded rustix v1.1.4
  Downloaded regex-automata v0.4.14
  Downloaded gimli v0.31.1
  Downloaded percentage v0.1.0
  Downloaded getrandom v0.2.17
  Downloaded litesvm v0.9.1
  Downloaded getrandom v0.1.16
  Downloaded typed-arena v2.0.2
  Downloaded serial_test_derive v3.5.0
  Downloaded solana-zk-token-sdk v3.1.14
  Downloaded num-bigint v0.4.6
  Downloaded memmap2 v0.9.10
  Downloaded regex-syntax v0.8.10
  Downloaded object v0.37.3
  Downloaded uriparse v0.6.4
  Downloaded spl-token-interface v2.0.0
  Downloaded spl-token-confidential-transfer-proof-extraction v0.5.1
  Downloaded solana-zk-sdk v4.0.0
  Downloaded object v0.36.7
  Downloaded spl-associated-token-account-interface v2.0.0
  Downloaded der v0.7.10
  Downloaded sha2 v0.9.9
  Downloaded serial_test v3.5.0
  Downloaded serde_with v3.20.0
  Downloaded rand_chacha v0.3.1
  Downloaded rand v0.8.6
  Downloaded libm v0.2.16
  Downloaded libafl_bolts v0.15.4
  Downloaded solana-zk-token-proof-program v3.1.14
  Downloaded solana-nonce v3.0.0
  Downloaded serde-big-array v0.5.1
  Downloaded rand_chacha v0.2.2
  Downloaded rand v0.7.3
  Downloaded paste v1.0.15
  Downloaded libafl_derive v0.15.4
  Downloaded hostname v0.4.2
  Downloaded hashbrown v0.15.5
  Downloaded getrandom v0.3.4
  Downloaded typed-builder v0.22.0
  Downloaded signature v2.2.0
  Downloaded polyval v0.6.2
  Downloaded libsecp256k1-core v0.2.2
  Downloaded hashbrown v0.13.2
  Downloaded solana-nonce-account v3.0.0
  Downloaded itertools v0.14.0
  Downloaded solana-fee-structure v3.0.0
  Downloaded solana-compute-budget-interface v3.0.0
  Downloaded sha3 v0.10.9
  Downloaded sec1 v0.7.3
  Downloaded rand_core v0.9.5
  Downloaded pbkdf2 v0.11.0
  Downloaded num_enum v0.7.6
  Downloaded num-complex v0.2.4
  Downloaded merlin v3.0.0
  Downloaded libsecp256k1 v0.6.0
  Downloaded getrandom v0.4.2
  Downloaded unreachable v1.0.0
  Downloaded spl-token-metadata-interface v0.8.0
  Downloaded solana-packet v3.0.0
  Downloaded solana-clock v3.0.1
  Downloaded num_enum_derive v0.7.6
  Downloaded num-derive v0.4.2
  Downloaded libsecp256k1-gen-genmult v0.2.1
  Downloaded hash32 v0.3.1
  Downloaded wide v0.7.33
  Downloaded stable_deref_trait v1.2.1
  Downloaded num-bigint v0.2.6
  Downloaded hex v0.4.3
  Downloaded group v0.13.0
  Downloaded typeid v1.0.3
  Downloaded twox-hash v1.6.3
  Downloaded spl-type-length-value v0.9.1
  Downloaded shlex v2.0.1
  Downloaded safe_arch v0.7.4
  Downloaded ruzstd v0.7.3
  Downloaded rustc-hash v2.1.2
  Downloaded linux-raw-sys v0.12.1
  Downloaded rustc-demangle v0.1.27
  Downloaded rand_core v0.5.1
  Downloaded miniz_oxide v0.8.9
  Downloaded libsecp256k1-gen-ecmult v0.2.1
  Downloaded spl-discriminator v0.5.2
  Downloaded solana-seed-phrase v3.0.0
  Downloaded solana-compute-budget v3.1.14
  Downloaded opaque-debug v0.3.1
  Downloaded mimalloc v0.1.52
  Downloaded memoffset v0.9.1
  Downloaded solana-zk-elgamal-proof-program v3.1.14
  Downloaded solana-zero-copy v1.1.1
  Downloaded serde_with_macros v3.20.0
  Downloaded foldhash v0.2.0
  Downloaded spl-token-group-interface v0.7.1
  Downloaded spl-token-2022-interface v2.1.0
  Downloaded spl-discriminator-syn v0.2.1
  Downloaded solana-address-lookup-table-interface v3.0.1
  Downloaded rfc6979 v0.4.0
  Downloaded qualifier_attr v0.2.2
  Downloaded qstring v0.7.2
  Downloaded num-rational v0.2.4
  Downloaded num-iter v0.1.45
  Downloaded num v0.2.1
  Downloaded meminterval v0.4.2
  Downloaded konst v0.2.20
  Downloaded itertools v0.10.5
  Downloaded hybrid-array v0.4.12
  Downloaded hmac v0.12.1
  Downloaded solana-svm-type-overrides v3.1.14
  Downloaded solana-svm-transaction v3.1.14
  Downloaded solana-svm-feature-set v3.1.14
  Downloaded pkcs8 v0.10.2
  Downloaded solana-stable-layout v3.0.0
  Downloaded solana-native-token v3.0.0
  Downloaded rustversion v1.0.22
  Downloaded num-integer v0.1.46
  Downloaded spl-token v9.0.0
  Downloaded spl-discriminator-derive v0.2.0
  Downloaded spki v0.7.3
  Downloaded solana-vote-interface v4.0.4
  Downloaded solana-system-program v3.1.14
  Downloaded solana-sbpf v0.13.1
  Downloaded itertools v0.13.0
  Downloaded solana-vote-program v3.1.14
  Downloaded solana-transaction v3.0.2
  Downloaded k256 v0.13.4
  Downloaded solana-svm-timings v3.1.14
  Downloaded solana-svm-measure v3.1.14
  Downloaded solana-short-vec v3.2.1
  Downloaded solana-serde-varint v3.0.1
  Downloaded solana-secp256k1-recover v3.1.1
  Downloaded solana-svm-log-collector v3.1.14
  Downloaded solana-program-runtime v3.1.14
  Downloaded solana-signature v3.1.0
  Downloaded solana-loader-v4-program v3.1.14
  Downloaded solana-instructions-sysvar v3.0.0
  Downloaded aho-corasick v1.1.4
  Downloaded solana-seed-derivable v3.0.0
  Downloaded solana-precompile-error v3.0.0
  Downloaded solana-poseidon v3.1.14
  Downloaded solana-loader-v4-interface v3.1.0
  Downloaded solana-loader-v3-interface v6.1.0
  Downloaded solana-keypair v3.1.0
  Downloaded solana-keccak-hasher v3.1.0
  Downloaded solana-instruction v3.1.0
  Downloaded solana-hash v3.1.0
  Downloaded solana-epoch-schedule v3.0.0
  Downloaded solana-compute-budget-instruction v3.1.14
  Downloaded solana-builtins v3.1.14
  Downloaded solana-bpf-loader-program v3.1.14
  Downloaded ed25519-dalek v2.2.0
  Downloaded crypto-bigint v0.5.5
  Downloaded cpp_demangle v0.4.5
  Downloaded const_panic v0.2.15
  Downloaded combine v3.8.1
  Downloaded cc v1.2.63
  Downloaded blake3 v1.8.5
  Downloaded base64 v0.12.3
  Downloaded backtrace v0.3.76
  Downloaded ark-ff v0.4.2
  Downloaded ark-ec v0.4.2
  Downloaded arbitrary-int v2.1.1
  Downloaded solana-curve25519 v3.1.14
  Downloaded solana-builtins-default-costs v3.1.14
  Downloaded solana-big-mod-exp v3.0.0
  Downloaded elliptic-curve v0.13.8
  Downloaded either v1.16.0
  Downloaded arbitrary v1.4.2
  Downloaded agave-syscalls v3.1.14
  Downloaded agave-feature-set v3.1.14
  Downloaded solana-borsh v3.0.2
  Downloaded solana-bn254 v3.2.1
  Downloaded siphasher v1.0.3
  Downloaded fastbloom v0.14.1
  Downloaded crc32fast v1.5.0
  Downloaded ark-poly v0.5.0
  Downloaded ark-poly v0.4.2
  Downloaded ark-ff v0.5.0
  Downloaded anchor-spl v1.0.2
  Downloaded ahash v0.8.12
  Downloaded aes v0.8.4
  Downloaded nix v0.31.3
  Downloaded five8 v0.2.1
  Downloaded ff v0.13.1
  Downloaded errno v0.3.14
  Downloaded erased-serde v0.4.10
  Downloaded educe v0.6.0
  Downloaded ed25519 v2.2.3
  Downloaded ecdsa v0.16.9
  Downloaded digest v0.11.3
  Downloaded derivative v2.2.0
  Downloaded ctutils v0.4.2
  Downloaded ctr v0.9.2
  Downloaded crypto-common v0.2.2
  Downloaded constant_time_eq v0.4.2
  Downloaded const_format_proc_macros v0.2.34
  Downloaded const_format v0.2.36
  Downloaded const-oid v0.9.6
  Downloaded cipher v0.4.4
  Downloaded bitbybit v1.4.0
  Downloaded base64ct v1.8.3
  Downloaded ascii v0.9.3
  Downloaded ark-ec v0.5.0
  Downloaded arbitrary-int v1.3.0
  Downloaded agave-reserved-account-keys v3.1.14
  Downloaded five8_core v0.1.2
  Downloaded embedded-io v0.6.1
  Downloaded block-buffer v0.12.0
  Downloaded bincode v2.0.1
  Downloaded allocator-api2 v0.2.21
  Downloaded aes-gcm-siv v0.11.1
  Downloaded eager v0.1.0
  Downloaded digest v0.9.0
  Downloaded derivation-path v0.2.0
  Downloaded ctor-proc-macro v0.0.7
  Downloaded cpufeatures v0.3.0
  Downloaded cmov v0.5.4
  Downloaded arrayref v0.3.9
  Downloaded ark-std v0.5.0
  Downloaded ark-bn254 v0.5.0
  Downloaded dtor-proc-macro v0.0.6
  Downloaded bincode_derive v2.0.1
  Downloaded ark-serialize-derive v0.5.0
  Downloaded ark-ff-macros v0.5.0
  Downloaded ark-bn254 v0.4.0
  Downloaded ark-serialize-derive v0.4.2
  Downloaded ppv-lite86 v0.2.21
  Downloaded flate2 v1.1.9
  Downloaded ark-serialize v0.5.0
  Downloaded ctrlc v3.5.2
  Downloaded ark-serialize v0.4.2
  Downloaded ark-ff-asm v0.5.0
  Downloaded ark-ff-asm v0.4.2
  Downloaded enum-iterator-derive v1.5.0
  Downloaded block-buffer v0.9.0
  Downloaded ark-std v0.4.0
  Downloaded byteorder v1.5.0
  Downloaded arrayvec v0.7.6
  Downloaded ansi_term v0.12.1
  Downloaded fs2 v0.4.3
  Downloaded ctor v0.6.3
  Downloaded enum-ordinalize-derive v4.3.2
  Downloaded derive_arbitrary v1.4.2
  Downloaded crunchy v0.2.4
  Downloaded ark-ff-macros v0.4.2
   Compiling proc-macro2 v1.0.106
   Compiling quote v1.0.45
   Compiling unicode-ident v1.0.24
   Compiling cfg-if v1.0.4
   Compiling version_check v0.9.5
   Compiling libc v0.2.186
   Compiling syn v2.0.117
   Compiling typenum v1.20.1
   Compiling getrandom v0.2.17
   Compiling generic-array v0.14.7
   Compiling rand_core v0.6.4
   Compiling subtle v2.6.1
   Compiling const-oid v0.9.6
   Compiling serde_core v1.0.228
   Compiling zeroize_derive v1.4.3
   Compiling zeroize v1.8.2
   Compiling crypto-common v0.1.7
   Compiling serde_derive v1.0.228
   Compiling block-buffer v0.10.4
   Compiling digest v0.10.7
   Compiling autocfg v1.5.1
   Compiling serde v1.0.228
   Compiling cfg_aliases v0.2.1
   Compiling hashbrown v0.17.1
   Compiling winnow v1.0.3
   Compiling equivalent v1.0.2
   Compiling indexmap v2.14.0
   Compiling toml_parser v1.1.2+spec-1.1.0
   Compiling toml_datetime v1.1.1+spec-1.1.0
   Compiling toml_edit v0.25.12+spec-1.1.0
   Compiling bytemuck_derive v1.10.2
   Compiling proc-macro-crate v3.5.0
   Compiling cpufeatures v0.2.17
   Compiling bytemuck v1.25.0
   Compiling num-traits v0.2.19
   Compiling borsh v1.6.1
   Compiling once_cell v1.21.4
   Compiling borsh-derive v1.6.1
   Compiling five8_core v1.0.0
   Compiling solana-sanitize v3.0.1
   Compiling sha2 v0.10.9
   Compiling semver v1.0.28
   Compiling rustc_version v0.4.1
   Compiling five8 v1.0.0
   Compiling solana-atomic-u64 v3.0.1
   Compiling solana-hash v4.3.0
   Compiling curve25519-dalek v4.1.3
   Compiling solana-program-error v3.0.1
   Compiling curve25519-dalek-derive v0.1.1
   Compiling solana-sha256-hasher v3.1.0
   Compiling sha2-const-stable v0.1.0
   Compiling five8_const v1.0.0
   Compiling zerocopy v0.8.50
   Compiling solana-address v2.6.0
   Compiling solana-instruction-error v2.3.0
   Compiling solana-sdk-ids v3.1.0
   Compiling solana-pubkey v4.2.0
   Compiling bincode v1.3.3
   Compiling solana-address v1.1.0
   Compiling solana-pubkey v3.0.0
   Compiling ppv-lite86 v0.2.21
   Compiling solana-instruction v3.1.0
   Compiling rand_chacha v0.3.1
   Compiling bs58 v0.5.1
   Compiling rand v0.8.6
   Compiling either v1.16.0
   Compiling solana-sysvar-id v3.1.0
   Compiling thiserror v2.0.18
   Compiling lazy_static v1.5.0
   Compiling thiserror-impl v2.0.18
   Compiling memchr v2.8.1
   Compiling num-integer v0.1.46
   Compiling solana-sdk-macro v3.0.1
   Compiling signature v2.2.0
   Compiling solana-program-memory v3.1.0
   Compiling syn v1.0.109
   Compiling solana-account-info v3.1.1
   Compiling base64 v0.22.1
   Compiling bitflags v2.12.1
   Compiling log v0.4.31
   Compiling thiserror v1.0.69
   Compiling getrandom v0.3.4
   Compiling ed25519 v2.2.3
   Compiling thiserror-impl v1.0.69
   Compiling five8_core v0.1.2
   Compiling five8 v0.2.1
   Compiling ed25519-dalek v2.2.0
   Compiling solana-transaction-error v3.0.0
   Compiling serde-big-array v0.5.1
   Compiling ahash v0.8.12
   Compiling solana-signature v3.1.0
   Compiling byteorder v1.5.0
   Compiling arrayvec v0.7.6
   Compiling feature-probe v0.1.1
   Compiling bv v0.11.1
   Compiling solana-signer v3.0.0
   Compiling solana-epoch-schedule v3.0.0
   Compiling find-msvc-tools v0.1.9
   Compiling shlex v2.0.1
   Compiling cc v1.2.63
   Compiling solana-clock v3.0.1
   Compiling solana-define-syscall v4.0.1
   Compiling solana-msg v3.1.0
   Compiling solana-program-entrypoint v3.1.1
   Compiling solana-fee-calculator v3.2.0
   Compiling solana-rent v3.1.0
   Compiling solana-slot-hashes v3.0.2
   Compiling hmac v0.12.1
   Compiling arrayref v0.3.9
   Compiling zmij v1.0.21
   Compiling solana-slot-history v3.0.1
   Compiling solana-epoch-rewards v3.0.2
   Compiling solana-last-restart-slot v3.0.1
   Compiling solana-sysvar v3.1.1
   Compiling itertools v0.12.1
   Compiling solana-hash v3.1.0
   Compiling serde_bytes v0.11.19
   Compiling hybrid-array v0.4.12
   Compiling num-bigint v0.4.6
   Compiling solana-serialize-utils v3.1.2
   Compiling cmov v0.5.4
   Compiling opaque-debug v0.3.1
   Compiling ctutils v0.4.2
   Compiling block-buffer v0.12.0
   Compiling crypto-common v0.2.2
   Compiling blake3 v1.8.5
   Compiling solana-system-interface v2.0.0
   Compiling rustc-demangle v0.1.27
   Compiling keccak v0.1.6
   Compiling paste v1.0.15
   Compiling digest v0.11.3
   Compiling solana-instructions-sysvar v3.0.0
   Compiling num-derive v0.4.2
   Compiling cpufeatures v0.3.0
   Compiling constant_time_eq v0.4.2
   Compiling sha3 v0.10.9
   Compiling solana-cpi v3.1.0
   Compiling num-bigint v0.2.6
   Compiling solana-short-vec v3.2.1
   Compiling rustversion v1.0.22
   Compiling solana-stable-layout v3.0.0
   Compiling num-rational v0.2.4
   Compiling num-complex v0.2.4
   Compiling pbkdf2 v0.11.0
   Compiling crunchy v0.2.4
   Compiling void v1.0.2
   Compiling unreachable v1.0.0
   Compiling solana-seed-phrase v3.0.0
   Compiling solana-message v3.0.1
   Compiling solana-stake-interface v2.0.2
   Compiling solana-loader-v3-interface v6.1.0
   Compiling solana-account v3.4.0
   Compiling inout v0.1.4
   Compiling solana-svm-feature-set v3.1.14
   Compiling ascii v0.9.3
   Compiling serde_json v1.0.150
   Compiling anyhow v1.0.102
   Compiling combine v3.8.1
   Compiling cipher v0.4.4
   Compiling solana-transaction v3.0.2
   Compiling hash32 v0.3.1
   Compiling num-iter v0.1.45
   Compiling enum-ordinalize-derive v4.3.2
   Compiling enum-iterator-derive v1.5.0
   Compiling itoa v1.0.18
   Compiling fnv v1.0.7
   Compiling percent-encoding v2.3.2
   Compiling qstring v0.7.2
   Compiling enum-iterator v1.5.0
   Compiling uriparse v0.6.4
   Compiling enum-ordinalize v4.3.2
   Compiling solana-sbpf v0.13.1
   Compiling num v0.2.1
   Compiling solana-svm-transaction v3.1.14
   Compiling ark-std v0.5.0
   Compiling solana-precompile-error v3.0.0
   Compiling universal-hash v0.5.1
   Compiling ark-serialize-derive v0.5.0
   Compiling eager v0.1.0
   Compiling derivation-path v0.2.0
   Compiling solana-fee-structure v3.0.0
   Compiling solana-derivation-path v3.0.0
   Compiling solana-svm-timings v3.1.14
   Compiling ark-serialize v0.5.0
   Compiling polyval v0.6.2
   Compiling solana-svm-callback v3.1.14
   Compiling solana-transaction-context v3.1.14
   Compiling percentage v0.1.0
   Compiling educe v0.6.0
   Compiling ctr v0.9.2
   Compiling aes v0.8.4
   Compiling ark-ff-macros v0.5.0
   Compiling ark-serialize-derive v0.4.2
   Compiling solana-svm-log-collector v3.1.14
   Compiling solana-svm-type-overrides v3.1.14
   Compiling ark-std v0.4.0
   Compiling itertools v0.13.0
   Compiling aead v0.5.2
   Compiling der v0.7.10
   Compiling ark-ff-asm v0.5.0
   Compiling solana-svm-measure v3.1.14
   Compiling allocator-api2 v0.2.21
   Compiling hashbrown v0.15.5
   Compiling solana-program-runtime v3.1.14
   Compiling ark-ff v0.5.0
   Compiling aes-gcm-siv v0.11.1
   Compiling ark-serialize v0.4.2
   Compiling solana-seed-derivable v3.0.0
   Compiling merlin v3.0.0
   Compiling ark-ff-macros v0.4.2
   Compiling ark-ff-asm v0.4.2
   Compiling derivative v2.2.0
   Compiling digest v0.9.0
   Compiling itertools v0.10.5
   Compiling num_enum_derive v0.7.6
   Compiling unicode-segmentation v1.13.3
   Compiling getrandom v0.1.16
   Compiling libsecp256k1-core v0.2.2
   Compiling heck v0.3.3
   Compiling num_enum v0.7.6
   Compiling ark-ff v0.4.2
   Compiling ark-poly v0.5.0
   Compiling spki v0.7.3
   Compiling hashbrown v0.13.2
   Compiling solana-curve25519 v3.1.14
   Compiling ff v0.13.1
   Compiling base16ct v0.2.0
   Compiling sec1 v0.7.3
   Compiling group v0.13.0
   Compiling pkcs8 v0.10.2
   Compiling ark-ec v0.5.0
   Compiling crypto-bigint v0.5.5
   Compiling ident_case v1.0.1
   Compiling strsim v0.11.1
   Compiling darling_core v0.23.0
   Compiling elliptic-curve v0.13.8
   Compiling rand_core v0.5.1
   Compiling ark-bn254 v0.5.0
   Compiling ark-poly v0.4.2
   Compiling anchor-syn v1.0.2
   Compiling ark-ec v0.4.2
   Compiling libsecp256k1-gen-genmult v0.2.1
   Compiling libsecp256k1-gen-ecmult v0.2.1
   Compiling solana-zk-sdk v4.0.0
   Compiling rfc6979 v0.4.0
   Compiling solana-program-option v3.1.0
   Compiling ecdsa v0.16.9
   Compiling libsecp256k1 v0.6.0
   Compiling ark-bn254 v0.4.0
   Compiling rand_chacha v0.2.2
   Compiling darling_macro v0.23.0
   Compiling agave-feature-set v3.1.14
   Compiling block-buffer v0.9.0
   Compiling sha2 v0.9.9
   Compiling darling v0.23.0
   Compiling rand v0.7.3
   Compiling light-poseidon v0.2.0
   Compiling k256 v0.13.4
   Compiling light-poseidon v0.4.0
   Compiling solana-packet v3.0.0
   Compiling solana-bincode v3.1.0
   Compiling base64 v0.12.3
   Compiling adler2 v2.0.1
   Compiling simd-adler32 v0.3.9
   Compiling miniz_oxide v0.8.9
   Compiling solana-poseidon v3.1.14
   Compiling solana-secp256k1-recover v3.1.1
   Compiling serde_with_macros v3.20.0
   Compiling solana-bn254 v3.2.1
   Compiling solana-keccak-hasher v3.1.0
   Compiling solana-blake3-hasher v3.1.0
   Compiling solana-big-mod-exp v3.0.0
   Compiling static_assertions v1.1.0
   Compiling smallvec v1.15.1
   Compiling rustix v1.1.4
   Compiling agave-syscalls v3.1.14
   Compiling serde_with v3.20.0
   Compiling spl-discriminator-syn v0.2.1
   Compiling solana-loader-v4-interface v3.1.0
   Compiling solana-nonce v3.0.0
   Compiling solana-program-pack v3.1.0
   Compiling solana-zero-copy v1.1.1
   Compiling solana-serde-varint v3.0.1
   Compiling qualifier_attr v0.2.2
   Compiling cfg_eval v0.1.2
   Compiling parking_lot_core v0.9.12
   Compiling utf8parse v0.2.2
   Compiling linux-raw-sys v0.12.1
   Compiling anstyle-parse v1.0.0
   Compiling solana-vote-interface v4.0.4
   Compiling solana-bpf-loader-program v3.1.14
   Compiling solana-nonce-account v3.0.0
   Compiling spl-discriminator-derive v0.2.0
   Compiling anchor-lang-idl-spec v0.1.0
   Compiling solana-keypair v3.1.0
   Compiling solana-borsh v3.0.2
   Compiling memoffset v0.9.1
   Compiling getrandom v0.4.2
   Compiling typeid v1.0.3
   Compiling anstyle-query v1.1.5
   Compiling object v0.37.3
   Compiling is_terminal_polyfill v1.70.2
   Compiling crc32fast v1.5.0
   Compiling scopeguard v1.2.0
   Compiling colorchoice v1.0.5
   Compiling anstyle v1.0.14
   Compiling anstream v1.0.0
   Compiling lock_api v0.4.14
   Compiling solana-vote-program v3.1.14
   Compiling anchor-lang-idl v0.1.2
   Compiling spl-discriminator v0.5.2
   Compiling solana-system-program v3.1.14
   Compiling solana-loader-v4-program v3.1.14
   Compiling terminal_size v0.4.4
   Compiling spl-pod v0.7.3
   Compiling solana-compute-budget-program v3.1.14
   Compiling nix v0.30.1
   Compiling clap_lex v1.1.0
   Compiling gimli v0.32.3
   Compiling heck v0.5.0
   Compiling keccak-const v0.2.0
   Compiling dtor-proc-macro v0.0.6
   Compiling libm v0.2.16
   Compiling erased-serde v0.4.10
   Compiling solana-define-syscall v3.0.0
   Compiling solana-invoke v0.5.0
   Compiling dtor v0.1.1
   Compiling clap_builder v4.6.0
   Compiling addr2line v0.25.1
   Compiling const-crypto v0.3.0
   Compiling clap_derive v4.6.1
   Compiling parking_lot v0.12.5
   Compiling anchor-attribute-program v1.0.2
   Compiling twox-hash v1.6.3
   Compiling anchor-attribute-error v1.0.2
   Compiling anchor-attribute-event v1.0.2
   Compiling anchor-derive-serde v1.0.2
   Compiling anchor-attribute-constant v1.0.2
   Compiling anchor-attribute-account v1.0.2
   Compiling anchor-derive-accounts v1.0.2
   Compiling solana-zk-token-sdk v3.1.14
   Compiling libafl_bolts v0.15.4
   Compiling anchor-derive-space v1.0.2
   Compiling anchor-attribute-access-control v1.0.2
   Compiling aho-corasick v1.1.4
   Compiling cobs v0.3.0
   Compiling solana-feature-gate-interface v3.1.0
   Compiling safe_arch v0.7.4
   Compiling serial_test_derive v3.5.0
   Compiling ctor-proc-macro v0.0.7
   Compiling unicode-xid v0.2.6
   Compiling foldhash v0.2.0
   Compiling object v0.36.7
   Compiling base64 v0.21.7
   Compiling regex-syntax v0.8.10
   Compiling konst_macro_rules v0.2.19
   Compiling virtue v0.0.18
   Compiling cpp_demangle v0.4.5
   Compiling bincode_derive v2.0.1
   Compiling konst v0.2.20
   Compiling regex-automata v0.4.14
   Compiling solana-zk-token-proof-program v3.1.14
   Compiling anchor-lang v1.0.2
   Compiling hashbrown v0.16.1
   Compiling const_format_proc_macros v0.2.34
   Compiling ctor v0.6.3
   Compiling serial_test v3.5.0
   Compiling wide v0.7.33
   Compiling postcard v1.1.3
   Compiling ruzstd v0.7.3
   Compiling flate2 v1.1.9
   Compiling backtrace v0.3.76
   Compiling uuid v1.23.2
   Compiling clap v4.6.1
   Compiling solana-builtins-default-costs v3.1.14
   Compiling spl-type-length-value v0.9.1
   Compiling spl-token-interface v2.0.0
   Compiling solana-zk-elgamal-proof-program v3.1.14
   Compiling solana-compute-budget v3.1.14
   Compiling libafl v0.15.4
   Compiling libmimalloc-sys v0.1.49
   Compiling solana-compute-budget-interface v3.0.0
   Compiling libafl_derive v0.15.4
   Compiling typed-builder-macro v0.22.0
   Compiling uds v0.4.2
   Compiling hostname v0.4.2
   Compiling typewit v1.15.2
   Compiling rand_core v0.9.5
   Compiling siphasher v1.0.3
   Compiling unty v0.0.4
   Compiling arbitrary-int v1.3.0
   Compiling tuple_list v0.1.3
   Compiling xxhash-rust v0.8.15
   Compiling bitbybit v1.4.0
   Compiling bincode v2.0.1
   Compiling fastbloom v0.14.1
   Compiling const_panic v0.2.15
   Compiling typed-builder v0.22.0
   Compiling solana-compute-budget-instruction v3.1.14
   Compiling solana-builtins v3.1.14
   Compiling spl-token-metadata-interface v0.8.0
   Compiling const_format v0.2.36
   Compiling regex v1.12.3
   Compiling spl-token-confidential-transfer-proof-extraction v0.5.1
   Compiling spl-token-group-interface v0.7.1
   Compiling spl-token-confidential-transfer-proof-generation v0.5.1
   Compiling solana-fee v3.1.14
   Compiling agave-reserved-account-keys v3.1.14
   Compiling solana-address-lookup-table-interface v3.0.1
   Compiling solana-system-interface v3.0.0
   Compiling itertools v0.14.0
   Compiling meminterval v0.4.2
   Compiling nix v0.31.3
   Compiling crucible-macro-utils v0.2.0 (/home/runner/work/cf-invariants-anchor/crucible/crates/crucible-macro-utils)
   Compiling fs2 v0.4.3
   Compiling wait-timeout v0.2.1
   Compiling memmap2 v0.9.10
   Compiling hex v0.4.3
   Compiling solana-native-token v3.0.0
   Compiling typed-arena v2.0.2
   Compiling ansi_term v0.12.1
   Compiling arbitrary-int v2.1.1
   Compiling gimli v0.31.1
   Compiling fallible-iterator v0.3.0
   Compiling addr2line v0.24.2
   Compiling litesvm v0.9.1
   Compiling spl-token-2022-interface v2.1.0
   Compiling spl-token v9.0.0
   Compiling spl-associated-token-account-interface v2.0.0
   Compiling rustc-hash v2.1.2
   Compiling crucible-test-context v0.2.0 (/home/runner/work/cf-invariants-anchor/crucible/crates/crucible-test-context)
warning: unused import: `anchor_lang::prelude::sysvar::SysvarId`
 --> /home/runner/work/cf-invariants-anchor/crucible/crates/crucible-test-context/src/lib.rs:1:5
  |
1 | use anchor_lang::prelude::sysvar::SysvarId;
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unused import: `solana_pubkey::Pubkey`
 --> /home/runner/work/cf-invariants-anchor/crucible/crates/crucible-test-context/src/instruction_builder.rs:6:5
  |
6 | use solana_pubkey::Pubkey;
  |     ^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `SvmSnapshot`
 --> /home/runner/work/cf-invariants-anchor/crucible/crates/crucible-test-context/src/snapshot/state_pool.rs:7:41
  |
7 | use super::svm_snapshot::{CompactDelta, SvmSnapshot, FINGERPRINT_BITS};
  |                                         ^^^^^^^^^^^

warning: unused imports: `EpochSchedule`, `SlotHashes`, and `SlotHistory`
 --> /home/runner/work/cf-invariants-anchor/crucible/crates/crucible-test-context/src/snapshot/svm_snapshot.rs:3:35
  |
3 | use anchor_lang::prelude::{Clock, EpochSchedule, SlotHashes, SlotHistory, StakeHistory};
  |                                   ^^^^^^^^^^^^^  ^^^^^^^^^^  ^^^^^^^^^^^

warning: use of deprecated struct `solana_sysvar::fees::Fees`: Please do not use, will no longer be available in the future
 --> /home/runner/work/cf-invariants-anchor/crucible/crates/crucible-test-context/src/snapshot/svm_snapshot.rs:9:26
  |
9 | use solana_sysvar::fees::Fees;
  |                          ^^^^
  |
  = note: `#[warn(deprecated)]` on by default

warning: use of deprecated struct `solana_sysvar::recent_blockhashes::RecentBlockhashes`: Please do not use, will no longer be available in the future
  --> /home/runner/work/cf-invariants-anchor/crucible/crates/crucible-test-context/src/snapshot/svm_snapshot.rs:11:40
   |
11 | use solana_sysvar::recent_blockhashes::RecentBlockhashes;
   |                                        ^^^^^^^^^^^^^^^^^

warning: use of deprecated struct `solana_sysvar::fees::Fees`: Please do not use, will no longer be available in the future
   --> /home/runner/work/cf-invariants-anchor/crucible/crates/crucible-test-context/src/snapshot/svm_snapshot.rs:404:13
    |
404 |             Fees::id(),
    |             ^^^^

warning: use of deprecated struct `solana_sysvar::recent_blockhashes::RecentBlockhashes`: Please do not use, will no longer be available in the future
   --> /home/runner/work/cf-invariants-anchor/crucible/crates/crucible-test-context/src/snapshot/svm_snapshot.rs:406:13
    |
406 |             RecentBlockhashes::id(),
    |             ^^^^^^^^^^^^^^^^^

warning: unused import: `ReadableAccount`
 --> /home/runner/work/cf-invariants-anchor/crucible/crates/crucible-test-context/src/snapshot/svm_snapshot.rs:6:31
  |
6 | use solana_account::{Account, ReadableAccount};
  |                               ^^^^^^^^^^^^^^^

warning: unused variable: `tp`
   --> /home/runner/work/cf-invariants-anchor/crucible/crates/crucible-test-context/src/snapshot/state_pool.rs:963:13
    |
963 |         let tp = self.total_picks.load(Ordering::Relaxed) as f64;
    |             ^^ help: if this is intentional, prefix it with an underscore: `_tp`
    |
    = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

warning: unused variable: `tp`
    --> /home/runner/work/cf-invariants-anchor/crucible/crates/crucible-test-context/src/snapshot/state_pool.rs:1027:13
     |
1027 |         let tp = self.total_picks.load(Ordering::Relaxed) as f64;
     |             ^^ help: if this is intentional, prefix it with an underscore: `_tp`

warning: unused variable: `tp`
    --> /home/runner/work/cf-invariants-anchor/crucible/crates/crucible-test-context/src/snapshot/state_pool.rs:1534:13
     |
1534 |         let tp = self.total_picks.load(Ordering::Relaxed) as f64;
     |             ^^ help: if this is intentional, prefix it with an underscore: `_tp`

warning: unused variable: `tp_f`
    --> /home/runner/work/cf-invariants-anchor/crucible/crates/crucible-test-context/src/snapshot/state_pool.rs:1744:13
     |
1744 |         let tp_f = tp as f64;
     |             ^^^^ help: if this is intentional, prefix it with an underscore: `_tp_f`

warning: unused variable: `default_kp`
    --> /home/runner/work/cf-invariants-anchor/crucible/crates/crucible-test-context/src/lib.rs:2201:13
     |
2201 |         let default_kp = Keypair::new();
     |             ^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_default_kp`

warning: field `ctx` is never read
 --> /home/runner/work/cf-invariants-anchor/crucible/crates/crucible-test-context/src/transaction_builder.rs:6:16
  |
5 | pub struct TransactionBuilder<'a> {
  |            ------------------ field in this struct
6 |     pub(crate) ctx: &'a mut TestContext,
  |                ^^^
  |
  = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

   Compiling anchor-spl v1.0.2
warning: `crucible-test-context` (lib) generated 15 warnings (run `cargo fix --lib -p crucible-test-context` to apply 9 suggestions)
   Compiling mimalloc v0.1.52
   Compiling crucible-fuzz-runtime v0.2.0 (/home/runner/work/cf-invariants-anchor/crucible/crates/crucible-fuzz-runtime)
   Compiling crucible-fuzz-macro v0.2.0 (/home/runner/work/cf-invariants-anchor/crucible/crates/crucible-fuzz-macro)
   Compiling crucible-invariant-macro v0.2.0 (/home/runner/work/cf-invariants-anchor/crucible/crates/crucible-invariant-macro)
warning: function `parse_features_from_content` is never used
    --> /home/runner/work/cf-invariants-anchor/crucible/crates/crucible-invariant-macro/src/lib.rs:1461:4
     |
1461 | fn parse_features_from_content(content: &str) -> Vec<String> {
     |    ^^^^^^^^^^^^^^^^^^^^^^^^^^^
     |
     = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: `crucible-invariant-macro` (lib) generated 1 warning
   Compiling derive_arbitrary v1.4.2
   Compiling arbitrary v1.4.2
   Compiling ctrlc v3.5.2
   Compiling vault_ref v0.1.0 (/home/runner/work/cf-invariants-anchor/cf-invariants-anchor/references/vault_ref/programs/vault_ref)
   Compiling crucible-fuzzer v0.2.0 (/home/runner/work/cf-invariants-anchor/crucible/crates/crucible-fuzzer)
   Compiling vault_ref_fuzz v0.1.0 (/home/runner/work/cf-invariants-anchor/cf-invariants-anchor/references/vault_ref/fuzz/vault_ref)
warning: variable does not need to be mutable
   --> src/main.rs:124:1
    |
124 | #[invariant_test]
    | ^^^^^^^^^^^^^^^^^ help: remove this `mut`
    |
    = note: `#[warn(unused_mut)]` (part of `#[warn(unused)]`) on by default
    = note: this warning originates in the macro `run_fuzz_loop` (in Nightly builds, run with -Z macro-backtrace for more info)

warning: variable does not need to be mutable
   --> src/main.rs:124:1
    |
124 | #[invariant_test]
    | ^^^^^^^^^^^^^^^^^ help: remove this `mut`
    |
    = note: this warning originates in the macro `run_fuzz_loop` (in Nightly builds, run with -Z macro-backtrace for more info)

warning: `vault_ref_fuzz` (bin "invariant_test") generated 2 warnings (run `cargo fix --bin "invariant_test" -p vault_ref_fuzz` to apply 2 suggestions)
    Finished `release` profile [optimized] target(s) in 4m 54s

  ______
  ╲    ╱   ___ ___ _   _  ___ ___ ___ _    ___
   ╲╱╲╱   / __| _ \ | | |/ __|_ _| _ ) |  | __|
   ╱╲╱╲  | (__|   / |_| | (__ | || _ \ |__| _|
  ╱    ╲  \___|_|_\\___/ \___|___|___/____|___|
  ▔▔▔▔▔▔

[FUZZ] Running with 30s timeout
[FUZZ] Crashes directory: /home/runner/work/cf-invariants-anchor/cf-invariants-anchor/references/vault_ref/fuzz/vault_ref/crashes/invariant_amount_conservation
[FUZZ] Max actions per iteration: 8
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 0, crashes: 0, executions: 1, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 5.0, ok: 4/5 (80.0%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 1, crashes: 0, executions: 1, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 5.0, ok: 4/5 (80.0%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 1, crashes: 0, executions: 2, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 3.0, ok: 4/6 (66.7%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 2, crashes: 0, executions: 2, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 3.0, ok: 4/6 (66.7%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 2, crashes: 0, executions: 3, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 2.7, ok: 6/8 (75.0%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 3, crashes: 0, executions: 3, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 2.7, ok: 6/8 (75.0%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 3, crashes: 0, executions: 4, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 2.2, ok: 7/9 (77.8%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 4, crashes: 0, executions: 4, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 2.2, ok: 7/9 (77.8%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 4, crashes: 0, executions: 6, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 2.7, ok: 13/16 (81.2%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 5, crashes: 0, executions: 6, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 2.7, ok: 13/16 (81.2%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 5, crashes: 0, executions: 7, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 3.4, ok: 19/24 (79.2%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 6, crashes: 0, executions: 7, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 3.4, ok: 19/24 (79.2%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 6, crashes: 0, executions: 8, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 4.0, ok: 27/32 (84.4%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 7, crashes: 0, executions: 8, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 4.0, ok: 27/32 (84.4%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 7, crashes: 0, executions: 11, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 4.6, ok: 41/51 (80.4%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 8, crashes: 0, executions: 11, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 4.6, ok: 41/51 (80.4%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 8, crashes: 0, executions: 13, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 5.1, ok: 55/66 (83.3%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 9, crashes: 0, executions: 13, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 5.1, ok: 55/66 (83.3%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 9, crashes: 0, executions: 18, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 5.4, ok: 85/97 (87.6%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 10, crashes: 0, executions: 18, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 5.4, ok: 85/97 (87.6%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 10, crashes: 0, executions: 59, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 5.5, ok: 258/324 (79.6%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 0s, clients: 1, corpus: 11, crashes: 0, executions: 59, exec/sec: 0.000, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 5.5, ok: 258/324 (79.6%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 1s, clients: 1, corpus: 11, crashes: 0, executions: 1959, exec/sec: 1.375k, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 6.2, ok: 10948/12186 (89.8%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 1s, clients: 1, corpus: 12, crashes: 0, executions: 1959, exec/sec: 1.375k, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 6.2, ok: 10948/12186 (89.8%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 6s, clients: 1, corpus: 12, crashes: 0, executions: 7873, exec/sec: 1.343k, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 6.1, ok: 43523/48173 (90.3%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 6s, clients: 1, corpus: 13, crashes: 0, executions: 7873, exec/sec: 1.343k, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 6.1, ok: 43523/48173 (90.3%), discovered: 2/2 actions
[FUZZ_PULSE] run time: 15s, clients: 1, corpus: 13, crashes: 0, executions: 20861, exec/sec: 1.390k, edges: 338/3282 (10.3%), branches: 321/1641 (19.6%), actions/exec: 6.1, ok: 114491/126268 (90.7%), discovered: 2/2 actions

[FUZZ] Timeout reached (30s). Exiting gracefully.
```
