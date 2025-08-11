#![forbid(unsafe_code)]

//! Nyx cryptography engine.
//!
//! This crate provides:
//! 1. Noise_Nyx handshake implementation (see [`noise`] module).
//! 2. HKDF wrappers with misuse-resistant label semantics.
//! 3. Optional Kyber1024 Post-Quantum support when built with `--features kyber`.
//! 4. HPKE (RFC 9180) wrapper utilities.

use zeroize::Zeroize;

pub mod noise;
pub mod kdf;
#[cfg(feature = "kyber")]
pub use noise::kyber;
#[cfg(feature = "hybrid")]
pub mod hybrid;
#[cfg(feature = "hybrid")]
pub use hybrid::PqAlgorithm;
pub mod aead;
pub mod keystore;
#[cfg(feature = "hpke")]
pub mod hpke;
pub mod pcr;

pub use kdf::KdfLabel;

/// Derive a new forward-secure session key from an existing key.
/// This provides post-compromise recovery (PCR) by hashing the current key
/// through HKDF with the dedicated `Rekey` label. The old key is zeroized
/// upon return.
pub fn pcr_rekey(old_key: &mut noise::SessionKey) -> noise::SessionKey {
    use kdf::{hkdf_expand, KdfLabel};
    let next_bytes = hkdf_expand(&old_key.0, KdfLabel::Rekey, 32);
    let mut out = [0u8; 32];
    out.copy_from_slice(&next_bytes);
    // zeroize old key material before returning
    old_key.0.zeroize();
    noise::SessionKey(out)
}
