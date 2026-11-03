//! Code pages, turning text in them into UTF-8, and finding which one a text is in.

mod detect;
mod languages;
mod polish;
mod tables;

use std::fmt;

pub use detect::{Detection, Detector};

/// A code page, found with [`by_name`].
#[derive(Debug)]
pub struct Codepage {
    name: &'static str,
    kind: Kind,
}

#[derive(Debug)]
enum Kind {
    Utf8,
    Utf16 { big_endian: bool },
    Utf32 { big_endian: bool },
    Table(&'static [u16; 128]),
}

/// Text with what could not be decoded replaced by U+FFFD.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LossyText {
    pub utf8: String,
    pub replaced: bool,
}

/// Bytes that mean nothing in a code page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidInput {
    /// Where the first of them is.
    pub offset: usize,
}

impl fmt::Display for InvalidInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid input at byte {}", self.offset)
    }
}

impl std::error::Error for InvalidInput {}

impl Codepage {
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Decodes bytes, all of which must be valid. A byte order mark stays, as U+FEFF.
    ///
    /// # Errors
    /// Tells where the first invalid bytes are.
    pub fn to_utf8(&self, bytes: &[u8]) -> Result<String, InvalidInput> {
        let mut text = String::with_capacity(bytes.len());
        self.decode(bytes, &mut text, |_, offset| Err(InvalidInput { offset }))?;
        Ok(text)
    }

    /// Decodes bytes, replacing what is invalid. A byte order mark stays, as U+FEFF.
    #[must_use]
    pub fn to_utf8_lossy(&self, bytes: &[u8]) -> LossyText {
        let mut utf8 = String::with_capacity(bytes.len());
        let mut replaced = false;
        let result = self.decode(bytes, &mut utf8, |text, _| {
            text.push(char::REPLACEMENT_CHARACTER);
            replaced = true;
            Ok(())
        });
        debug_assert!(result.is_ok());
        LossyText { utf8, replaced }
    }

    /// Decodes bytes, leaving out what is invalid.
    fn without_invalid(&self, bytes: &[u8]) -> String {
        let mut text = String::with_capacity(bytes.len());
        let result = self.decode(bytes, &mut text, |_, _| Ok(()));
        debug_assert!(result.is_ok());
        text
    }

    /// Adds the decoded bytes to `text`, and asks `invalid` what to do at those that are not.
    fn decode(
        &self,
        bytes: &[u8],
        text: &mut String,
        mut invalid: impl FnMut(&mut String, usize) -> Result<(), InvalidInput>,
    ) -> Result<(), InvalidInput> {
        match self.kind {
            Kind::Utf8 => {
                let mut offset = 0;
                for chunk in bytes.utf8_chunks() {
                    text.push_str(chunk.valid());
                    offset += chunk.valid().len();
                    if !chunk.invalid().is_empty() {
                        invalid(text, offset)?;
                        offset += chunk.invalid().len();
                    }
                }
            }
            Kind::Utf16 { big_endian } => {
                let chunks = bytes.chunks_exact(2);
                let tail = chunks.remainder().len();
                let units = chunks.map(|unit| {
                    let unit = [unit[0], unit[1]];
                    if big_endian {
                        u16::from_be_bytes(unit)
                    } else {
                        u16::from_le_bytes(unit)
                    }
                });
                let mut offset = 0;
                for decoded in char::decode_utf16(units) {
                    match decoded {
                        Ok(character) => {
                            text.push(character);
                            offset += character.len_utf16() * 2;
                        }
                        Err(_) => {
                            invalid(text, offset)?;
                            offset += 2;
                        }
                    }
                }
                if tail != 0 {
                    invalid(text, offset)?;
                }
            }
            Kind::Utf32 { big_endian } => {
                let chunks = bytes.chunks_exact(4);
                let tail = chunks.remainder().len();
                let mut offset = 0;
                for unit in chunks {
                    let unit = [unit[0], unit[1], unit[2], unit[3]];
                    let value = if big_endian {
                        u32::from_be_bytes(unit)
                    } else {
                        u32::from_le_bytes(unit)
                    };
                    match char::from_u32(value) {
                        Some(character) => text.push(character),
                        None => invalid(text, offset)?,
                    }
                    offset += 4;
                }
                if tail != 0 {
                    invalid(text, offset)?;
                }
            }
            Kind::Table(table) => {
                for (offset, &byte) in bytes.iter().enumerate() {
                    if byte < 128 {
                        text.push(char::from(byte));
                        continue;
                    }
                    let value = table[usize::from(byte - 128)];
                    match char::from_u32(u32::from(value)) {
                        Some(character)
                            if value != 0 && character != char::REPLACEMENT_CHARACTER =>
                        {
                            text.push(character);
                        }
                        _ => invalid(text, offset)?,
                    }
                }
            }
        }
        Ok(())
    }
}

const fn table(name: &'static str, table: &'static [u16; 128]) -> Codepage {
    Codepage {
        name,
        kind: Kind::Table(table),
    }
}

static CODEPAGES: &[Codepage] = &[
    Codepage {
        name: "utf8",
        kind: Kind::Utf8,
    },
    Codepage {
        name: "utf16",
        kind: Kind::Utf16 { big_endian: false },
    },
    Codepage {
        name: "utf16be",
        kind: Kind::Utf16 { big_endian: true },
    },
    Codepage {
        name: "utf32",
        kind: Kind::Utf32 { big_endian: false },
    },
    Codepage {
        name: "utf32be",
        kind: Kind::Utf32 { big_endian: true },
    },
    table("iso-8859-1", &tables::ISO_8859_1),
    table("iso-8859-2", &tables::ISO_8859_2),
    table("iso-8859-3", &tables::ISO_8859_3),
    table("iso-8859-4", &tables::ISO_8859_4),
    table("iso-8859-5", &tables::ISO_8859_5),
    table("iso-8859-6", &tables::ISO_8859_6),
    table("iso-8859-7", &tables::ISO_8859_7),
    table("iso-8859-8", &tables::ISO_8859_8),
    table("iso-8859-9", &tables::ISO_8859_9),
    table("iso-8859-10", &tables::ISO_8859_10),
    table("iso-8859-11", &tables::ISO_8859_11),
    table("iso-8859-13", &tables::ISO_8859_13),
    table("iso-8859-14", &tables::ISO_8859_14),
    table("iso-8859-15", &tables::ISO_8859_15),
    table("iso-8859-16", &tables::ISO_8859_16),
    table("cp1250", &tables::CP1250),
    table("cp1251", &tables::CP1251),
    table("cp1252", &tables::CP1252),
    table("cp1253", &tables::CP1253),
    table("cp1254", &tables::CP1254),
    table("cp1255", &tables::CP1255),
    table("cp1256", &tables::CP1256),
    table("cp1257", &tables::CP1257),
    table("cp1258", &tables::CP1258),
    table("cp874", &tables::CP874),
    table("cp437", &tables::CP437),
    table("cp737", &tables::CP737),
    table("cp775", &tables::CP775),
    table("cp850", &tables::CP850),
    table("cp852", &tables::CP852),
    table("cp855", &tables::CP855),
    table("cp856", &tables::CP856),
    table("cp857", &tables::CP857),
    table("cp860", &tables::CP860),
    table("cp861", &tables::CP861),
    table("cp862", &tables::CP862),
    table("cp863", &tables::CP863),
    table("cp864", &tables::CP864),
    table("cp865", &tables::CP865),
    table("cp866", &tables::CP866),
    table("cp869", &tables::CP869),
    table("koi8-r", &tables::KOI8_R),
    table("koi8-u", &tables::KOI8_U),
    table("mac-roman", &tables::MAC_ROMAN),
    table("mac-greek", &tables::MAC_GREEK),
    table("mac-cyrillic", &tables::MAC_CYRILLIC),
    table("mac-iceland", &tables::MAC_ICELAND),
    table("mac-turkish", &tables::MAC_TURKISH),
    table("mac-latin2", &tables::MAC_LATIN2),
    table("apple-arabic", &tables::APPLE_ARABIC),
    table("apple-celtic", &tables::APPLE_CELTIC),
    table("apple-centeur", &tables::APPLE_CENTEUR),
    table("apple-croatian", &tables::APPLE_CROATIAN),
    table("apple-cyrillic", &tables::APPLE_CYRILLIC),
    table("apple-devanaga", &tables::APPLE_DEVANAGA),
    table("apple-farsi", &tables::APPLE_FARSI),
    table("apple-gaelic", &tables::APPLE_GAELIC),
    table("apple-greek", &tables::APPLE_GREEK),
    table("apple-gujarati", &tables::APPLE_GUJARATI),
    table("apple-gurmukhi", &tables::APPLE_GURMUKHI),
    table("apple-hebrew", &tables::APPLE_HEBREW),
    table("apple-iceland", &tables::APPLE_ICELAND),
    table("apple-inuit", &tables::APPLE_INUIT),
    table("apple-roman", &tables::APPLE_ROMAN),
    table("apple-romanian", &tables::APPLE_ROMANIAN),
    table("apple-thai", &tables::APPLE_THAI),
    table("apple-turkish", &tables::APPLE_TURKISH),
    table("apple-ukraine", &tables::APPLE_UKRAINE),
    table("gsm", &tables::GSM),
    table("ebcdic", &tables::EBCDIC),
    table("atarist", &tables::ATARIST),
    table("kz1048", &tables::KZ1048),
    table("mazovia", &polish::MAZOVIA),
    table("dhn", &polish::DHN),
    table("csk", &polish::CSK),
    table("cyfromat", &polish::CYFROMAT),
    table("iinte-isis", &polish::IINTE_ISIS),
    table("iea-swierk", &polish::IEA_SWIERK),
    table("logic-pl", &polish::LOGIC),
    table("microvex", &polish::MICROVEX),
    table("ventura-pl", &polish::VENTURA),
    table("fido-mazovia", &polish::FIDO_MAZOVIA),
    table("smc", &polish::SMC),
    table("elwro-junior", &polish::ELWRO_JUNIOR),
    table("atari-calamus", &polish::ATARI_CALAMUS),
    table("coreldraw-pl", &polish::COREL_DRAW),
    table("corel2-pl", &polish::COREL_2),
    table("atm-pl", &polish::ATM),
    table("texpl", &polish::TEXPL),
    table("amigapl", &polish::AMIGA_PL),
    table("amiga-fat", &polish::AMIGA_FAT),
    table("amiga-xjp", &polish::AMIGA_XJP),
    table("mac-pl", &polish::MAC_PL),
];

/// All code pages.
#[must_use]
pub fn codepages() -> &'static [Codepage] {
    CODEPAGES
}

/// Finds a code page by its name, in any case.
#[must_use]
pub fn by_name(name: &str) -> Option<&'static Codepage> {
    CODEPAGES
        .iter()
        .find(|codepage| codepage.name.eq_ignore_ascii_case(name))
}
