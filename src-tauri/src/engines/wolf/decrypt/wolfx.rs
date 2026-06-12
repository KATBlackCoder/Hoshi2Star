//! WolfX (Wolf RPG v3.5+ Pro) archive decryption — seam only, not implemented.
//!
//! Starting with Wolf RPG Editor v3.5, Pro games encrypt `.wolf` archives with
//! ChaCha20 and store only a *hash* of the protection key, so the key cannot be
//! recovered from the archive (unlike the legacy XOR scheme, where
//! [`super::legacy_xor::guess_key_v6`] exploits known-plaintext header bytes).
//!
//! Decision (2026-06-12): native ChaCha20/WolfX support is out of scope.
//! The supported workflow is a manual pre-step — the user decrypts with
//! UberWolf (<https://github.com/Sinflower/UberWolf>), then opens the resulting
//! plain `Data/` directory in Hoshi2Star. UberWolf is NOT bundled (no confirmed
//! license; Windows-only binary). Everything downstream of decryption
//! (`extractor`, `injector`, `format` parsers) consumes plain `.mps`/`.dat`
//! files and needs no changes if native support is ever added here.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum WolfXError {
    /// WolfX decryption is intentionally not implemented (see module docs).
    #[error(
        "this archive uses WolfX encryption (Wolf RPG v3.5+ Pro), which Hoshi2Star \
         does not decrypt. Decrypt the game with UberWolf first, then open the \
         resulting Data/ directory directly"
    )]
    NotSupported,
}

/// Seam for future WolfX decryption. Always returns
/// [`WolfXError::NotSupported`] with user guidance — callers surface this
/// message when [`super::legacy_xor`] reports
/// [`super::legacy_xor::DecryptorError::PossibleWolfX`].
pub fn decrypt(_data: &[u8]) -> Result<Vec<u8>, WolfXError> {
    Err(WolfXError::NotSupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decrypt_returns_guidance() {
        let err = decrypt(&[0u8; 16]).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("UberWolf"));
        assert!(msg.contains("WolfX"));
    }
}
