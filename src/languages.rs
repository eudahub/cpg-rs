//! The languages that have a model, each with the code pages it has been written in, the most
//! common first. Generated once from `languages.txt` of the C++ cpg.

/// A language with its model.
pub struct Language {
    pub iso: &'static str,
    pub charsets: &'static [&'static str],
    pub model: &'static [u8],
}

#[rustfmt::skip]
pub static LANGUAGES: &[Language] = &[
    Language { iso: "ar", charsets: &["iso-8859-6", "cp1256", "cp864"], model: include_bytes!("../data/models/ar.ngram") },
    Language { iso: "be", charsets: &["iso-8859-5", "cp1251", "cp866", "mac-cyrillic"], model: include_bytes!("../data/models/be.ngram") },
    Language { iso: "bg", charsets: &["iso-8859-5", "cp1251", "cp855"], model: include_bytes!("../data/models/bg.ngram") },
    Language { iso: "cz", charsets: &["iso-8859-2", "cp1250"], model: include_bytes!("../data/models/cz.ngram") },
    Language { iso: "da", charsets: &["iso-8859-1", "iso-8859-15", "cp1252", "mac-roman"], model: include_bytes!("../data/models/da.ngram") },
    Language { iso: "de", charsets: &["iso-8859-1", "iso-8859-15", "cp1252", "mac-roman"], model: include_bytes!("../data/models/de.ngram") },
    Language { iso: "el", charsets: &["iso-8859-7", "cp1253"], model: include_bytes!("../data/models/el.ngram") },
    Language { iso: "en", charsets: &["iso-8859-1", "cp1252", "mac-roman"], model: include_bytes!("../data/models/en.ngram") },
    Language { iso: "eo", charsets: &["iso-8859-3"], model: include_bytes!("../data/models/eo.ngram") },
    Language { iso: "es", charsets: &["iso-8859-1", "iso-8859-15", "cp1252", "mac-roman"], model: include_bytes!("../data/models/es.ngram") },
    Language { iso: "et", charsets: &["iso-8859-4", "iso-8859-13", "cp1257"], model: include_bytes!("../data/models/et.ngram") },
    Language { iso: "fi", charsets: &["iso-8859-1", "iso-8859-15", "cp1252", "mac-roman"], model: include_bytes!("../data/models/fi.ngram") },
    Language { iso: "fr", charsets: &["iso-8859-1", "iso-8859-15", "cp1252", "mac-roman"], model: include_bytes!("../data/models/fr.ngram") },
    Language { iso: "he", charsets: &["iso-8859-8", "cp1255"], model: include_bytes!("../data/models/he.ngram") },
    Language { iso: "hr", charsets: &["iso-8859-2", "cp1250"], model: include_bytes!("../data/models/hr.ngram") },
    Language { iso: "hu", charsets: &["iso-8859-2", "cp1250"], model: include_bytes!("../data/models/hu.ngram") },
    Language { iso: "it", charsets: &["iso-8859-1", "iso-8859-15", "cp1252", "mac-roman"], model: include_bytes!("../data/models/it.ngram") },
    Language { iso: "lt", charsets: &["iso-8859-13", "cp1257", "iso-8859-4"], model: include_bytes!("../data/models/lt.ngram") },
    Language { iso: "lv", charsets: &["iso-8859-13", "cp1257", "iso-8859-4"], model: include_bytes!("../data/models/lv.ngram") },
    Language { iso: "mk", charsets: &["iso-8859-5", "cp1251", "mac-cyrillic", "cp855"], model: include_bytes!("../data/models/mk.ngram") },
    Language { iso: "nl", charsets: &["iso-8859-1", "cp1252", "mac-roman"], model: include_bytes!("../data/models/nl.ngram") },
    Language { iso: "pl", charsets: &["cp1250", "iso-8859-2", "iso-8859-16", "cp852", "mac-latin2", "mazovia", "dhn", "csk", "cyfromat", "iinte-isis", "iea-swierk", "logic-pl", "microvex", "ventura-pl", "fido-mazovia", "smc", "elwro-junior", "atari-calamus", "coreldraw-pl", "corel2-pl", "atm-pl", "texpl", "amigapl", "amiga-fat", "amiga-xjp", "mac-pl"], model: include_bytes!("../data/models/pl.ngram") },
    Language { iso: "pt", charsets: &["iso-8859-1", "iso-8859-15", "cp1252", "mac-roman"], model: include_bytes!("../data/models/pt.ngram") },
    Language { iso: "ro", charsets: &["iso-8859-2", "cp1250"], model: include_bytes!("../data/models/ro.ngram") },
    Language { iso: "ru", charsets: &["iso-8859-5", "cp1251", "koi8-r", "mac-cyrillic", "cp866", "cp855"], model: include_bytes!("../data/models/ru.ngram") },
    Language { iso: "sk", charsets: &["iso-8859-2", "cp1250"], model: include_bytes!("../data/models/sk.ngram") },
    Language { iso: "sl", charsets: &["iso-8859-2", "cp1250"], model: include_bytes!("../data/models/sl.ngram") },
    Language { iso: "sr", charsets: &["iso-8859-5", "cp1251", "mac-cyrillic", "cp855"], model: include_bytes!("../data/models/sr.ngram") },
    Language { iso: "th", charsets: &["iso-8859-11", "cp874"], model: include_bytes!("../data/models/th.ngram") },
    Language { iso: "tr", charsets: &["iso-8859-3", "iso-8859-9", "cp1254"], model: include_bytes!("../data/models/tr.ngram") },
    Language { iso: "uk", charsets: &["koi8-u", "cp1251"], model: include_bytes!("../data/models/uk.ngram") },
    Language { iso: "vi", charsets: &["cp1258"], model: include_bytes!("../data/models/vi.ngram") },
];
