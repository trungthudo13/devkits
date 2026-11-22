use super::*;
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Tool {
    Json,
    Xml,
    Sql,
    Password,
    Qr,
    Regex,
    Pdf,
    Cryptographic,
    Keygen,
    Base64,
    TextDiff,
}
impl Tool {
    pub(super) const ALL: [Self; 11] = [
        Self::Json,
        Self::Sql,
        Self::Xml,
        Self::Regex,
        Self::TextDiff,
        Self::Password,
        Self::Qr,
        Self::Pdf,
        Self::Cryptographic,
        Self::Keygen,
        Self::Base64,
    ];
    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Json => "JSON Editor",
            Self::Xml => "XML Editor",
            Self::Sql => "SQL Editor",
            Self::Password => "Random Password",
            Self::Qr => "QR Generator",
            Self::Regex => "Regex Editor",
            Self::Pdf => "PDF Extraction",
            Self::Cryptographic => "Cryptographic",
            Self::Keygen => "Keygen",
            Self::Base64 => "Base64",
            Self::TextDiff => "Text Diff",
        }
    }
    pub(super) fn description(self) -> &'static str {
        match self {
            Self::Json => "Format, validate and explore JSON documents.",
            Self::Xml => "Format and validate XML documents.",
            Self::Sql => "Format SQL and check syntax across dialects.",
            Self::Regex => "Test expressions, inspect matches and transform text.",
            Self::Password => "Generate secure passwords locally.",
            Self::Qr => "Create QR codes and export PNG images.",
            Self::Pdf => "Extract images and text, crop regions and run offline OCR.",
            Self::Cryptographic => "Hash, verify passwords, encrypt and decrypt text locally.",
            Self::Keygen => "Generate RSA, age and AES keys locally.",
            Self::Base64 => "Encode, decode and generate Base64 encryption keys.",
            Self::TextDiff => {
                "Compare text lines with additions on the left and removals on the right."
            }
        }
    }
    pub(super) fn icon(self) -> &'static str {
        match self {
            Self::Json => icons::BRACKETS_CURLY,
            Self::Xml => icons::CODE,
            Self::Sql => icons::DATABASE,
            Self::Regex => icons::ASTERISK,
            Self::Password => icons::KEY,
            Self::Qr => icons::QR_CODE,
            Self::Pdf => icons::FILE_PDF,
            Self::Cryptographic => icons::LOCK_KEY,
            Self::Keygen => icons::KEY,
            Self::Base64 => icons::CODE,
            Self::TextDiff => icons::GIT_DIFF,
        }
    }
    pub(super) fn lang(self) -> Option<Language> {
        match self {
            Self::Json => Some(Language::Json),
            Self::Xml => Some(Language::Xml),
            Self::Sql => Some(Language::Sql),
            _ => None,
        }
    }
    pub(super) fn index(self) -> usize {
        match self {
            Self::Json => 0,
            Self::Xml => 1,
            Self::Sql => 2,
            _ => 0,
        }
    }
}
