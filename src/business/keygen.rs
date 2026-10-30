//! Local key generation, with interoperable public/private encodings.
use age::secrecy::ExposeSecret;
use base64::{engine::general_purpose::STANDARD, Engine};
use rand::RngCore;
use rsa::{
    pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding},
    RsaPrivateKey, RsaPublicKey,
};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyKind {
    Rsa(usize),
    Age,
    Aes256,
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct KeyPair {
    pub public: String,
    pub private: String,
}

pub fn generate(kind: KeyKind) -> Result<KeyPair, String> {
    match kind {
        KeyKind::Rsa(bits) => {
            if ![2048, 3072, 4096].contains(&bits) {
                return Err("RSA size must be 2048, 3072 or 4096 bits.".into());
            }
            let private = RsaPrivateKey::new(&mut crate::adapter::random::secure_rng(), bits)
                .map_err(|_| "RSA key generation failed.")?;
            let public = RsaPublicKey::from(&private)
                .to_public_key_pem(LineEnding::LF)
                .map_err(|_| "Could not encode the RSA public key.")?;
            let private = private
                .to_pkcs8_pem(LineEnding::LF)
                .map_err(|_| "Could not encode the RSA private key.")?;
            Ok(KeyPair {
                public,
                private: private.to_string(),
            })
        }
        KeyKind::Age => {
            let identity = age::x25519::Identity::generate();
            Ok(KeyPair {
                public: identity.to_public().to_string(),
                private: identity.to_string().expose_secret().to_owned(),
            })
        }
        KeyKind::Aes256 => Ok(KeyPair {
            public: String::new(),
            private: aes_key()?,
        }),
    }
}

pub fn aes_key() -> Result<String, String> {
    let mut key = zeroize::Zeroizing::new([0u8; 32]);
    crate::adapter::random::secure_rng()
        .try_fill_bytes(key.as_mut())
        .map_err(|_| "Operating-system randomness is unavailable.")?;
    Ok(STANDARD.encode(key.as_ref()))
}
