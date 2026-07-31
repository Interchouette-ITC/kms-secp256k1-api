//! Domain errors for KMS API services and HTTP handlers.
//!
//! Library / service boundary uses [`KmsError`] (`thiserror`). HTTP handlers map
//! `Display` into JSON `{"error": ...}` (status mapping can tighten later).

use thiserror::Error;

/// Crate result alias for service and KMS client code.
pub type Result<T> = std::result::Result<T, KmsError>;

/// Typed failures from key ops, signing, verification, AWS KMS, and crypto/WASM.
///
/// `# Errors` rustdoc on `pub` APIs should name the relevant variants / conditions.
#[derive(Debug, Error)]
pub enum KmsError {
    /// Wrong blockchain / feature mode for the requested operation.
    #[error("Only {mode} mode is supported")]
    ModeMismatch {
        /// Expected mode name (e.g. `Ethereum`, `Casper`, `Cosmos`).
        mode: &'static str,
    },

    /// Production path requires a supported mode (or testing).
    #[error("Unsupported KMS mode and not in testing")]
    UnsupportedMode,

    /// Alias / address / public key lookup failed in local store or KMS.
    #[error("Key not found")]
    KeyNotFound,

    /// Public key missing from keystore entry.
    #[error("Public key not found")]
    PublicKeyNotFound,

    /// AWS alias metadata missing.
    #[error("KeyMetadata not found for alias {alias}")]
    AliasNotFound {
        /// Alias that was looked up.
        alias: String,
    },

    /// KMS response lacked a key id.
    #[error("No KeyId found")]
    MissingKeyId,

    /// Key creation produced an empty public key.
    #[error("No public key generated")]
    EmptyPublicKey,

    /// Hex decode / format failure.
    #[error("Invalid hex: {0}")]
    InvalidHex(String),

    /// Transaction hash hex is invalid.
    #[error("Invalid transaction hash hex: {0}")]
    InvalidTxHashHex(String),

    /// Transaction hash is not 32 bytes.
    #[error("Transaction hash must be 32 bytes")]
    TxHashWrongLength,

    /// Signature bytes failed hex decode.
    #[error("Invalid hex in signature")]
    InvalidSignatureHex,

    /// Signature length is not the expected size.
    #[error("Invalid signature length (expected {expected} bytes)")]
    InvalidSignatureLength {
        /// Expected byte length.
        expected: usize,
    },

    /// Request or intermediate JSON failed to parse.
    #[error("Failed to parse input JSON: {0}")]
    ParseJson(String),

    /// Transaction body / params parse failure (chain-specific wording preserved in message).
    #[error("{0}")]
    ParseTransaction(String),

    /// Transaction shape is not supported.
    #[error("Unsupported transaction format")]
    UnsupportedTxFormat,

    /// Signing failed after inputs were accepted.
    #[error("Signing failed: {0}")]
    SigningFailed(String),

    /// Local or WASM verification returned false / failed.
    #[error("Signature verification failed")]
    VerificationFailed,

    /// Verification failed with detail.
    #[error("Signature verification failed: {0}")]
    VerificationFailedDetail(String),

    /// Fresh signature failed self-check.
    #[error("Generated signature failed verification")]
    PostSignVerifyFailed,

    /// AWS KMS (or mock client) operation failed.
    #[error("{0}")]
    Kms(String),

    /// Crypto / WASM helper failure (often from `Box<dyn Error>` at the boundary).
    #[error("{0}")]
    Crypto(String),

    /// Cosmos REST / account / proto pipeline failure.
    #[error("{0}")]
    Cosmos(String),

    /// Catch-all for remaining stringly errors; prefer a named variant when touching call sites.
    #[error("{0}")]
    Msg(String),
}

impl KmsError {
    /// Build [`KmsError::Msg`] from any displayable value.
    #[must_use]
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Msg(s.into())
    }

    /// Map a `Box<dyn Error>` (crypto/WASM) into [`KmsError::Crypto`].
    #[must_use]
    pub fn from_dyn(err: impl std::fmt::Display) -> Self {
        Self::Crypto(err.to_string())
    }
}

impl From<String> for KmsError {
    fn from(value: String) -> Self {
        Self::Msg(value)
    }
}

impl From<&str> for KmsError {
    fn from(value: &str) -> Self {
        Self::Msg(value.to_string())
    }
}
