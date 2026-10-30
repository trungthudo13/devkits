//! Text hashing and authenticated encryption. No UI or file I/O.
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{
    password_hash::SaltString, Algorithm, Argon2, Params, PasswordHash, PasswordHasher,
    PasswordVerifier, Version,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use rand::RngCore;
use rsa::{
    pkcs8::{DecodePrivateKey, DecodePublicKey},
    traits::PublicKeyParts,
    Oaep, RsaPrivateKey, RsaPublicKey,
};
use sha2::{Digest, Sha256, Sha512};
use std::io::{Read, Write};
use zeroize::Zeroizing;

pub const MAX_INPUT: usize = 2 * 1024 * 1024;
const MAX_KEY: usize = 16 * 1024;
const AES_PREFIX: &str = "devkits:aes256gcm:v1:";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HashAlgorithm {
    Md5,
    Sha256,
    Sha512,
    Argon2id,
}

#[derive(Clone, Copy, Debug)]
pub struct ArgonOptions {
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}
impl Default for ArgonOptions {
    fn default() -> Self {
        Self {
            memory_kib: 19456,
            iterations: 2,
            parallelism: 1,
        }
    }
}
impl ArgonOptions {
    fn params(self) -> Result<Params, String> {
        if !(8..=262144).contains(&self.memory_kib)
            || !(1..=10).contains(&self.iterations)
            || !(1..=8).contains(&self.parallelism)
        {
            return Err(
                "Argon2id limits: memory 8–262144 KiB, iterations 1–10, parallelism 1–8.".into(),
            );
        }
        Params::new(self.memory_kib, self.iterations, self.parallelism, Some(32))
            .map_err(|_| "Argon2id memory must be at least 8 × parallelism KiB.".into())
    }
}

fn bounded(input: &str) -> Result<(), String> {
    if input.len() > MAX_INPUT {
        Err("Input exceeds the 2 MiB limit.".into())
    } else {
        Ok(())
    }
}
fn key_bounded(key: &str) -> Result<(), String> {
    if key.len() > MAX_KEY {
        Err("Key exceeds the 16 KiB limit.".into())
    } else {
        Ok(())
    }
}

pub fn hash(
    algorithm: HashAlgorithm,
    input: &str,
    options: ArgonOptions,
) -> Result<String, String> {
    bounded(input)?;
    Ok(match algorithm {
        HashAlgorithm::Md5 => format!("{:x}", md5::Md5::digest(input.as_bytes())),
        HashAlgorithm::Sha256 => format!("{:x}", Sha256::digest(input.as_bytes())),
        HashAlgorithm::Sha512 => format!("{:x}", Sha512::digest(input.as_bytes())),
        HashAlgorithm::Argon2id => {
            let mut salt = [0u8; 16];
            crate::adapter::random::secure_rng()
                .try_fill_bytes(&mut salt)
                .map_err(|_| "Operating-system randomness is unavailable.")?;
            let salt = SaltString::encode_b64(&salt).map_err(|_| "Could not encode salt.")?;
            Argon2::new(Algorithm::Argon2id, Version::V0x13, options.params()?)
                .hash_password(input.as_bytes(), &salt)
                .map_err(|_| "Argon2id hashing failed.")?
                .to_string()
        }
    })
}

pub fn verify_argon2id(input: &str, encoded: &str) -> Result<bool, String> {
    bounded(input)?;
    if encoded.len() > 1024 {
        return Err("PHC hash is too long.".into());
    }
    let parsed = PasswordHash::new(encoded.trim()).map_err(|_| "Invalid Argon2id PHC string.")?;
    if parsed.algorithm.as_str() != "argon2id"
        || parsed.version != Some(19)
        || parsed.salt.is_none()
        || parsed.hash.is_none()
    {
        return Err("Expected a complete $argon2id$v=19$ PHC hash.".into());
    }
    // Bound work from untrusted PHC parameters before allocating memory.
    let param = |name| {
        parsed
            .params
            .get_decimal(name)
            .ok_or_else(|| "Missing Argon2id parameters.".to_string())
    };
    ArgonOptions {
        memory_kib: param("m")?,
        iterations: param("t")?,
        parallelism: param("p")?,
    }
    .params()?;
    match Argon2::default().verify_password(input.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(_) => Err("Invalid or unsupported Argon2id PHC hash.".into()),
    }
}

fn aes_cipher(key: &str) -> Result<Aes256Gcm, String> {
    key_bounded(key)?;
    let bytes = Zeroizing::new(
        STANDARD
            .decode(key.trim())
            .map_err(|_| "AES key must be Base64.")?,
    );
    Aes256Gcm::new_from_slice(&bytes)
        .map_err(|_| "AES-256 requires a 32-byte key (Base64 encoded).".into())
}
pub fn aes_encrypt(input: &str, key: &str) -> Result<String, String> {
    bounded(input)?;
    let cipher = aes_cipher(key)?;
    let mut nonce = [0u8; 12];
    crate::adapter::random::secure_rng()
        .try_fill_bytes(&mut nonce)
        .map_err(|_| "Operating-system randomness is unavailable.")?;
    let encrypted = cipher
        .encrypt(Nonce::from_slice(&nonce), input.as_bytes())
        .map_err(|_| "AES encryption failed.")?;
    let mut payload = nonce.to_vec();
    payload.extend(encrypted);
    Ok(format!("{AES_PREFIX}{}", STANDARD.encode(payload)))
}
pub fn aes_decrypt(input: &str, key: &str) -> Result<String, String> {
    // The envelope is larger than the original UTF-8 plaintext.
    if input.len() > MAX_INPUT * 2 {
        return Err("Encrypted input is too large.".into());
    }
    let cipher = aes_cipher(key)?;
    let encoded = input
        .trim()
        .strip_prefix(AES_PREFIX)
        .ok_or("Expected a DevKits AES-256-GCM v1 envelope.")?;
    let payload = STANDARD
        .decode(encoded)
        .map_err(|_| "Invalid envelope Base64.")?;
    if payload.len() < 28 {
        return Err("Truncated AES envelope.".into());
    }
    let decrypted = cipher
        .decrypt(Nonce::from_slice(&payload[..12]), &payload[12..])
        .map_err(|_| "Decryption failed: wrong key or modified ciphertext.")?;
    plaintext(decrypted)
}
fn plaintext(bytes: Vec<u8>) -> Result<String, String> {
    if bytes.len() > MAX_INPUT {
        return Err("Decrypted text exceeds 2 MiB.".into());
    }
    String::from_utf8(bytes).map_err(|_| "Decrypted data is not UTF-8 text.".into())
}
fn rsa_size(key: &RsaPublicKey) -> Result<(), String> {
    if !(2048..=4096).contains(&key.n().bits()) {
        Err("RSA keys must be between 2048 and 4096 bits.".into())
    } else {
        Ok(())
    }
}
pub fn rsa_encrypt(input: &str, public_pem: &str) -> Result<String, String> {
    bounded(input)?;
    key_bounded(public_pem)?;
    let key = RsaPublicKey::from_public_key_pem(public_pem.trim())
        .map_err(|_| "Expected an RSA public key in SPKI PEM format (BEGIN PUBLIC KEY).")?;
    rsa_size(&key)?;
    let max = key.size() - 2 * 32 - 2;
    if input.len() > max {
        return Err(format!("RSA-OAEP accepts at most {max} UTF-8 bytes with this key. Use AES or age for longer text."));
    }
    let bytes = key
        .encrypt(
            &mut crate::adapter::random::secure_rng(),
            Oaep::new::<Sha256>(),
            input.as_bytes(),
        )
        .map_err(|_| "RSA encryption failed.")?;
    Ok(STANDARD.encode(bytes))
}
pub fn rsa_decrypt(input: &str, private_pem: &str) -> Result<String, String> {
    if input.len() > 1024 {
        return Err("RSA ciphertext is too long.".into());
    }
    key_bounded(private_pem)?;
    let key = RsaPrivateKey::from_pkcs8_pem(private_pem.trim()).map_err(|_| {
        "Expected an unencrypted RSA private key in PKCS#8 PEM format (BEGIN PRIVATE KEY)."
    })?;
    rsa_size(&RsaPublicKey::from(&key))?;
    key.validate().map_err(|_| "Invalid RSA private key.")?;
    let bytes = STANDARD
        .decode(input.trim())
        .map_err(|_| "RSA ciphertext must be Base64.")?;
    if bytes.len() != key.size() {
        return Err("RSA ciphertext length does not match the key.".into());
    }
    let decrypted = key
        .decrypt_blinded(
            &mut crate::adapter::random::secure_rng(),
            Oaep::new::<Sha256>(),
            &bytes,
        )
        .map_err(|_| "Decryption failed: wrong key or modified ciphertext.")?;
    plaintext(decrypted)
}

pub fn age_encrypt(input: &str, recipient: &str) -> Result<String, String> {
    bounded(input)?;
    key_bounded(recipient)?;
    let recipient: age::x25519::Recipient = recipient
        .trim()
        .parse()
        .map_err(|_| "Expected an age X25519 recipient beginning with age1.")?;
    let encryptor =
        age::Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))
            .map_err(|_| "Invalid age recipient.")?;
    let mut output = Vec::new();
    let armor = age::armor::ArmoredWriter::wrap_output(&mut output, age::armor::Format::AsciiArmor)
        .map_err(|_| "Could not initialize age armor.")?;
    let mut writer = encryptor
        .wrap_output(armor)
        .map_err(|_| "Could not initialize age encryption.")?;
    writer
        .write_all(input.as_bytes())
        .map_err(|_| "age encryption failed.")?;
    writer
        .finish()
        .and_then(|armor| armor.finish())
        .map_err(|_| "Could not finish age encryption.")?;
    String::from_utf8(output).map_err(|_| "Invalid age armor output.".into())
}
pub fn age_decrypt(input: &str, identity: &str) -> Result<String, String> {
    if input.contains("ENC[AES256_GCM,") {
        return Err("This is a SOPS document, not a raw age message. Select SOPS + age and paste the complete document including its sops metadata.".into());
    }
    if input.len() > MAX_INPUT * 2 {
        return Err("Encrypted input is too large.".into());
    }
    key_bounded(identity)?;
    let identity: age::x25519::Identity = identity
        .trim()
        .parse()
        .map_err(|_| "Expected an age identity beginning with AGE-SECRET-KEY-1.")?;
    let reader = age::armor::ArmoredReader::new(input.as_bytes());
    let decryptor = age::Decryptor::new(reader).map_err(|_| "Invalid age armored message.")?;
    if decryptor.is_scrypt() {
        return Err(
            "This tool expects age recipient encryption, not a passphrase-encrypted file.".into(),
        );
    }
    let reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|_| "Decryption failed: wrong age identity or modified ciphertext.")?;
    let mut bytes = Vec::new();
    reader
        .take((MAX_INPUT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "Decryption failed: truncated or modified age ciphertext.")?;
    plaintext(bytes)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SopsFormat {
    Yaml,
    Json,
    Dotenv,
    Ini,
}
impl SopsFormat {
    pub fn name(self) -> &'static str {
        match self {
            Self::Yaml => "yaml",
            Self::Json => "json",
            Self::Dotenv => "dotenv",
            Self::Ini => "ini",
        }
    }
}

/// Delegate SOPS document handling and MAC verification to the official CLI.
pub fn sops_transform(
    input: &str,
    key: &str,
    format: SopsFormat,
    decrypt: bool,
) -> Result<String, String> {
    if input.len() > 8 * 1024 * 1024 {
        return Err("SOPS input exceeds 8 MiB.".into());
    }
    if input.trim().is_empty() {
        return Err("Paste the complete SOPS document, including metadata.".into());
    }
    key_bounded(key)?;
    let lines: Vec<_> = key
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    if lines.is_empty() {
        return Err(
            "Provide an age identity for decryption or an age recipient for encryption.".into(),
        );
    }
    if decrypt {
        for line in &lines {
            line.parse::<age::x25519::Identity>().map_err(|_| "Expected AGE-SECRET-KEY-1… identities, one per line. Comments from age-keygen are accepted.")?;
        }
    } else {
        for line in &lines {
            line.parse::<age::x25519::Recipient>()
                .map_err(|_| "Expected age1… recipients, one per line.")?;
        }
    }
    let normalized = Zeroizing::new(lines.join(if decrypt { "\n" } else { "," }));
    crate::adapter::sops::transform(input, &normalized, format.name(), decrypt)
}
