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
}
impl Tool {
    pub(super) const ALL: [Self; 7] = [
        Self::Json,
        Self::Sql,
        Self::Xml,
        Self::Regex,
        Self::Password,
        Self::Qr,
        Self::Pdf,
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
