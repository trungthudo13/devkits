use base64::{engine::general_purpose::STANDARD, Engine};
use self_devkits::business::{
    cryptographic::*,
    keygen::{self, KeyKind},
};

#[test]
fn digest_vectors_and_exact_whitespace() {
    let options = ArgonOptions::default();
    for (algorithm, expected) in [
        (HashAlgorithm::Md5, "900150983cd24fb0d6963f7d28e17f72"),
        (HashAlgorithm::Sha256, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
        (HashAlgorithm::Sha512, "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"),
    ] { assert_eq!(hash(algorithm, "abc", options).unwrap(), expected); }
    assert_eq!(
        hash(HashAlgorithm::Md5, "", options).unwrap(),
        "d41d8cd98f00b204e9800998ecf8427e"
    );
    assert_ne!(
        hash(HashAlgorithm::Sha256, "abc", options).unwrap(),
        hash(HashAlgorithm::Sha256, "abc\n", options).unwrap()
    );
    assert!(hash(HashAlgorithm::Sha256, &"x".repeat(MAX_INPUT + 1), options).is_err());
}

#[test]
fn argon2id_has_random_salts_and_verifies_phc_parameters() {
    let options = ArgonOptions {
        memory_kib: 1024,
        iterations: 1,
        parallelism: 1,
    };
    let a = hash(HashAlgorithm::Argon2id, "mật khẩu 🔐", options).unwrap();
    let b = hash(HashAlgorithm::Argon2id, "mật khẩu 🔐", options).unwrap();
    assert!(a.starts_with("$argon2id$v=19$m=1024,t=1,p=1$"));
    assert_ne!(a, b);
    assert!(verify_argon2id("mật khẩu 🔐", &a).unwrap());
    assert!(!verify_argon2id("wrong", &a).unwrap());
    for bad in [
        a.replace("m=1024", "m=4294967295"),
        a.replace("t=1", "t=99"),
        a.replace("argon2id", "argon2i"),
        "invalid".into(),
        "$argon2id$v=19$m=8,t=1,p=1".into(),
    ] {
        assert!(verify_argon2id("password", &bad).is_err());
    }
    assert!(hash(
        HashAlgorithm::Argon2id,
        "",
        ArgonOptions {
            memory_kib: 8,
            parallelism: 8,
            ..options
        }
    )
    .is_err());
}

#[test]
fn aes_authenticates_key_nonce_and_ciphertext() {
    let key = keygen::aes_key().unwrap();
    assert_eq!(STANDARD.decode(&key).unwrap().len(), 32);
    for input in ["", "Xin chào 🔐\n  giữ khoảng trắng", "abc"] {
        let encrypted = aes_encrypt(input, &key).unwrap();
        assert_eq!(aes_decrypt(&encrypted, &key).unwrap(), input);
        assert_ne!(aes_encrypt(input, &key).unwrap(), encrypted);
        assert!(aes_decrypt(&encrypted, &keygen::aes_key().unwrap()).is_err());
        let (prefix, encoded) = encrypted.rsplit_once(':').unwrap();
        let payload = STANDARD.decode(encoded).unwrap();
        for index in [0, payload.len() - 1] {
            let mut tampered = payload.clone();
            tampered[index] ^= 1;
            assert!(aes_decrypt(&format!("{prefix}:{}", STANDARD.encode(tampered)), &key).is_err());
        }
    }
    assert!(aes_encrypt("abc", "not-base64").is_err());
    assert!(aes_encrypt("abc", &STANDARD.encode([0; 16])).is_err());
    assert!(aes_decrypt("devkits:aes256gcm:v1:AA==", &key).is_err());
    assert!(aes_decrypt("other:v1:AA==", &key).is_err());
    // AES-256-GCM empty plaintext vector: all-zero key and nonce.
    let mut vector = vec![0u8; 12];
    vector.extend([
        0x53, 0x0f, 0x8a, 0xfb, 0xc7, 0x45, 0x36, 0xb9, 0xa9, 0x63, 0xb4, 0xf1, 0xc4, 0xcb, 0x73,
        0x8b,
    ]);
    assert_eq!(
        aes_decrypt(
            &format!("devkits:aes256gcm:v1:{}", STANDARD.encode(vector)),
            &STANDARD.encode([0; 32])
        )
        .unwrap(),
        ""
    );
}

#[test]
fn age_keys_and_armored_messages_roundtrip() {
    let keys = keygen::generate(KeyKind::Age).unwrap();
    assert!(keys.public.starts_with("age1"));
    assert!(keys.private.starts_with("AGE-SECRET-KEY-1"));
    for input in ["", "Xin chào age 🔐\n", &"x".repeat(70000)] {
        let encrypted = age_encrypt(input, &keys.public).unwrap();
        assert!(encrypted.starts_with("-----BEGIN AGE ENCRYPTED FILE-----"));
        assert_eq!(age_decrypt(&encrypted, &keys.private).unwrap(), input);
        let other = keygen::generate(KeyKind::Age).unwrap();
        assert!(age_decrypt(&encrypted, &other.private).is_err());
        let mut tampered = encrypted.into_bytes();
        let index = tampered.len() / 2;
        tampered[index] = if tampered[index] == b'A' { b'B' } else { b'A' };
        assert!(age_decrypt(&String::from_utf8(tampered).unwrap(), &keys.private).is_err());
    }
    assert!(age_encrypt("abc", "bad recipient").is_err());
    assert!(age_decrypt("bad data", &keys.private).is_err());
}

#[test]
fn rsa_keys_oaep_limits_and_authentication() {
    let keys = keygen::generate(KeyKind::Rsa(2048)).unwrap();
    assert!(keys.public.starts_with("-----BEGIN PUBLIC KEY-----"));
    assert!(keys.private.starts_with("-----BEGIN PRIVATE KEY-----"));
    for input in ["", "Xin chào RSA 🔐", &"a".repeat(190)] {
        let encrypted = rsa_encrypt(input, &keys.public).unwrap();
        assert_eq!(rsa_decrypt(&encrypted, &keys.private).unwrap(), input);
        let mut bytes = STANDARD.decode(&encrypted).unwrap();
        bytes[0] ^= 1;
        assert!(rsa_decrypt(&STANDARD.encode(bytes), &keys.private).is_err());
    }
    assert!(rsa_encrypt(&"a".repeat(191), &keys.public).is_err());
    assert!(rsa_encrypt(&"🔐".repeat(48), &keys.public).is_err());
    assert!(rsa_encrypt("abc", "bad key").is_err());
    assert!(rsa_decrypt("AA==", &keys.private).is_err());
    assert!(keygen::generate(KeyKind::Rsa(1024)).is_err());
}

fn cli(program: &str, args: &[&str], input: &[u8]) -> Vec<u8> {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{program}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
fn age_cli_interoperability_both_directions() {
    if std::process::Command::new("age")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("Skipping age interoperability: age CLI is unavailable.");
        return;
    }
    let keys = keygen::generate(KeyKind::Age).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let identity = dir.path().join("identity.txt");
    std::fs::write(&identity, format!("{}\n", keys.private)).unwrap();
    let input = "UTF-8 interop 🔐\n";
    let encrypted = age_encrypt(input, &keys.public).unwrap();
    assert_eq!(
        cli(
            "age",
            &["--decrypt", "--identity", identity.to_str().unwrap()],
            encrypted.as_bytes()
        ),
        input.as_bytes()
    );
    let external = cli(
        "age",
        &["--encrypt", "--armor", "--recipient", &keys.public],
        input.as_bytes(),
    );
    assert_eq!(
        age_decrypt(std::str::from_utf8(&external).unwrap(), &keys.private).unwrap(),
        input
    );
}

#[test]
fn openssl_rsa_oaep_interoperability_both_directions() {
    if std::process::Command::new("openssl")
        .arg("version")
        .output()
        .is_err()
    {
        eprintln!("Skipping RSA interoperability: OpenSSL is unavailable.");
        return;
    }
    let keys = keygen::generate(KeyKind::Rsa(2048)).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let private = dir.path().join("private.pem");
    let public = dir.path().join("public.pem");
    std::fs::write(&private, &keys.private).unwrap();
    std::fs::write(&public, &keys.public).unwrap();
    let input = "RSA interop 🔐";
    let encrypted = STANDARD
        .decode(rsa_encrypt(input, &keys.public).unwrap())
        .unwrap();
    let options = [
        "-pkeyopt",
        "rsa_padding_mode:oaep",
        "-pkeyopt",
        "rsa_oaep_md:sha256",
        "-pkeyopt",
        "rsa_mgf1_md:sha256",
    ];
    let mut args = vec!["pkeyutl", "-decrypt", "-inkey", private.to_str().unwrap()];
    args.extend(options);
    assert_eq!(cli("openssl", &args, &encrypted), input.as_bytes());
    let mut args = vec![
        "pkeyutl",
        "-encrypt",
        "-pubin",
        "-inkey",
        public.to_str().unwrap(),
    ];
    args.extend(options);
    let encrypted = cli("openssl", &args, input.as_bytes());
    assert_eq!(
        rsa_decrypt(&STANDARD.encode(encrypted), &keys.private).unwrap(),
        input
    );
}

#[test]
fn sops_age_documents_roundtrip_and_reject_wrong_keys_and_modified_data() {
    if std::process::Command::new("sops")
        .arg("decrypt")
        .arg("--help")
        .output()
        .is_err()
    {
        eprintln!("Skipping SOPS integration: sops CLI is unavailable.");
        return;
    }
    let keys = keygen::generate(KeyKind::Age).unwrap();
    let wrong = keygen::generate(KeyKind::Age).unwrap();
    let keyfile = format!(
        "# created: test\n# public key: {}\n{}\n",
        keys.public, keys.private
    );
    for (format, input) in [
        (
            SopsFormat::Yaml,
            "token: secret-value\nnested:\n  greeting: Xin chào\n",
        ),
        (
            SopsFormat::Json,
            "{\"token\":\"secret-value\",\"nested\":{\"greeting\":\"Xin chào\"}}",
        ),
        (
            SopsFormat::Dotenv,
            "TOKEN=secret-value\nGREETING=Xin chào\n",
        ),
        (
            SopsFormat::Ini,
            "[app]\ntoken=secret-value\ngreeting=Xin chào\n",
        ),
    ] {
        let encrypted = sops_transform(input, &keys.public, format, false).unwrap();
        assert!(encrypted.contains("ENC[AES256_GCM,"));
        assert!(!encrypted.contains("secret-value"));
        let output = sops_transform(&encrypted, &keyfile, format, true).unwrap();
        assert!(output.contains("secret-value") && output.contains("Xin chào"));
        assert!(!output.contains("ENC[AES256_GCM,"));
        assert!(sops_transform(&encrypted, &wrong.private, format, true).is_err());
        let tampered = encrypted.replacen("ENC[AES256_GCM,data:", "ENC[AES256_GCM,data:A", 1);
        assert!(sops_transform(&tampered, &keys.private, format, true).is_err());
        assert!(age_decrypt(&encrypted, &keys.private)
            .unwrap_err()
            .contains("SOPS"));
    }
    // More than a pipe buffer in both directions must not deadlock.
    let input = format!("large: {}\n", "x".repeat(100_000));
    let encrypted = sops_transform(&input, &keys.public, SopsFormat::Yaml, false).unwrap();
    let output = sops_transform(&encrypted, &keys.private, SopsFormat::Yaml, true).unwrap();
    assert!(output.contains(&"x".repeat(100_000)));
}

#[test]
fn sops_rejects_empty_invalid_keys_and_oversized_input_before_launch() {
    for key in ["", "# comment only", "age1notvalid", "AGE-SECRET-KEY-1bad"] {
        assert!(sops_transform("token: value", key, SopsFormat::Yaml, true).is_err());
    }
    let keys = keygen::generate(KeyKind::Age).unwrap();
    assert!(sops_transform("", &keys.private, SopsFormat::Yaml, true).is_err());
    assert!(sops_transform(
        &"x".repeat(8 * 1024 * 1024 + 1),
        &keys.private,
        SopsFormat::Yaml,
        true
    )
    .is_err());
}
