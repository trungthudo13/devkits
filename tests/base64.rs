use base64::{engine::general_purpose::STANDARD, Engine};
use self_devkits::business::{base64 as service, cryptographic};

#[test]
fn base64_text_vectors_unicode_and_unpadded_input() {
    for (plain, encoded) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foobar", "Zm9vYmFy"),
    ] {
        assert_eq!(service::encode(plain, false).unwrap(), encoded);
        let decoded = service::decode(encoded, false).unwrap();
        assert_eq!(decoded.value, plain);
        assert_eq!(decoded.byte_count, plain.len());
        assert!(decoded.is_utf8);
    }
    let text = "Xin chào 🔐\n  ";
    let encoded = service::encode(text, false).unwrap();
    assert_eq!(service::decode(&encoded, false).unwrap().value, text);
    assert_eq!(service::decode(" Zm 8\n", false).unwrap().value, "fo");
}

#[test]
fn binary_keys_url_safe_and_malformed_input() {
    let standard = service::decode("+/8=", false).unwrap();
    let url = service::decode("-_8", true).unwrap();
    assert_eq!(standard.value, "fbff");
    assert_eq!(url.value, standard.value);
    assert_eq!(url.byte_count, 2);
    assert!(!url.is_utf8);
    for invalid in ["!", "Z", "Zg=", "Zg===", "Zh==", "Zm=v", "-_8="] {
        assert!(service::decode(invalid, false).is_err(), "{invalid}");
    }
    assert!(service::decode("+/8=", true).is_err());
    assert!(service::encode(&"x".repeat(service::MAX_TEXT + 1), false).is_err());
    assert!(service::decode(&"x".repeat(service::MAX_TEXT * 2 + 1), false).is_err());
}

#[test]
fn generated_encryption_keys_have_requested_byte_length_and_work_with_aes() {
    for length in [16, 24, 32, 64] {
        let a = service::encryption_key(length).unwrap();
        let b = service::encryption_key(length).unwrap();
        assert_ne!(a, b);
        assert_eq!(STANDARD.decode(&a).unwrap().len(), length);
    }
    let key = service::encryption_key(32).unwrap();
    assert_eq!(key.len(), 44);
    let encrypted = cryptographic::aes_encrypt("test", &key).unwrap();
    assert_eq!(
        cryptographic::aes_decrypt(&encrypted, &key).unwrap(),
        "test"
    );
    for invalid in [0, 8, 31, 128, usize::MAX] {
        assert!(service::encryption_key(invalid).is_err());
    }
}
