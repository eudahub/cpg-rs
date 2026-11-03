//! Finding the code page and language of a text, with models of the characters of 32 languages.
//!
//! Each candidate decoding gets a probability of being the right one. Decodings that give the
//! same text share one probability. UTF-16 and UTF-32 are recognized by themselves, while valid
//! UTF-8 competes with the code pages, as for example Polish "Ół" in cp1250 is valid UTF-8 too.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};

use miniz_oxide::inflate::decompress_to_vec_zlib_with_limit;
use unicode_general_category::{GeneralCategory as Category, get_general_category};

use crate::by_name;
use crate::languages::{LANGUAGES, Language};

/// The width of one character in the keys of the models.
const SHIFT: u32 = 21;
/// The size of each of the three pieces of a large text that get sampled.
const SAMPLE_PART: usize = 2048;
/// How sure a recognizer of UTF-16 or UTF-32 must be, out of 100.
const MINIMUM_UNICODE_CONFIDENCE: i32 = 80;
/// The model's log likelihood ratio is divided by this. Neighbouring characters are not
/// independent and rare ones mislead the model, so the raw ratio is overconfident.
const EVIDENCE_DIVISOR: f64 = 8.0;
/// Nats for UTF-8 for each multi-byte sequence, which rarely arises by chance in other text.
const UTF8_EVIDENCE: f64 = 2.0;
/// Nats against a decoding for each lowercase letter directly followed by an uppercase one, as in
/// "przenieŚc". It is typical of a wrong code page, and models of lowercase text cannot see it.
const CASE_PENALTY: f64 = 2.0;
/// Nats against a code page for each level of [`rank`] above 1, as exotic ones are rarer.
const RARITY_PENALTY: f64 = 0.5;

/// A candidate for the code page of a text.
#[derive(Clone, Debug, PartialEq)]
pub struct Detection {
    pub codepage: &'static str,
    /// From 0 to 1: how likely the model finds the text, or how sure a Unicode recognizer is.
    pub score: f64,
    /// How common the code page is: the lower, the more common.
    pub rank: i32,
    /// The language, or empty when nothing is known about it.
    pub language: &'static str,
    /// How likely the model of the language finds the text.
    pub language_score: f64,
    /// The probability that this gives the right text, the same for all that decode alike.
    pub confidence: f64,
    /// The probability of the language.
    pub language_confidence: f64,
    /// Candidates with the same number decode the text alike.
    pub decoding: i32,
}

/// Counts of character sequences of one length, sorted by the sequence.
struct Counts {
    keys: Box<[u64]>,
    counts: Box<[u32]>,
}

impl Counts {
    fn get(&self, key: u64) -> u32 {
        self.keys
            .binary_search(&key)
            .map_or(0, |index| self.counts[index])
    }
}

/// How often characters and their sequences occur in a language.
struct Profile {
    language: &'static Language,
    single: Counts,
    pairs: Counts,
    triples: Counts,
    total: u64,
}

/// Reads the numbers of a model, which is part of the program and so cannot be damaged.
struct ModelReader<'a> {
    bytes: &'a [u8],
}

impl ModelReader<'_> {
    fn number(&mut self) -> u64 {
        let mut value = 0;
        for index in 0..10 {
            let byte = self.bytes[0];
            self.bytes = &self.bytes[1..];
            value |= u64::from(byte & 127) << (7 * index);
            if byte & 128 == 0 {
                break;
            }
        }
        value
    }

    fn counts(&mut self) -> Counts {
        let size = usize::try_from(self.number()).expect("a model fits in memory");
        let mut keys = Vec::with_capacity(size);
        let mut counts = Vec::with_capacity(size);
        let mut key = 0;
        for _ in 0..size {
            key += self.number();
            keys.push(key);
            counts.push(u32::try_from(self.number()).expect("counts come from a small corpus"));
        }
        Counts {
            keys: keys.into(),
            counts: counts.into(),
        }
    }
}

fn load_profile(language: &'static Language) -> Profile {
    let mut file = ModelReader {
        bytes: language
            .model
            .strip_prefix(b"CPG_NGRAM 3\n")
            .expect("a model in the known format"),
    };
    let plain_size = usize::try_from(file.number()).expect("a model fits in memory");
    let _compressed_size = file.number();
    let plain = decompress_to_vec_zlib_with_limit(file.bytes, plain_size)
        .expect("a model in the known format");
    let mut reader = ModelReader { bytes: &plain };
    let _retained_percent = reader.number();
    let _language_count = reader.number();
    let name_size = usize::try_from(reader.number()).expect("a short name");
    reader.bytes = &reader.bytes[name_size..];
    let total = reader.number();
    Profile {
        language,
        single: reader.counts(),
        pairs: reader.counts(),
        triples: reader.counts(),
        total,
    }
}

/// What a text is scored on: lowercase letters, with one word boundary for all punctuation,
/// digits and spaces. Controls, undefined bytes and unusual symbols stay as evidence.
fn tokens(text: &str) -> Vec<char> {
    let mut result = Vec::with_capacity(text.len());
    for character in text.chars() {
        if character == '\u{FEFF}' {
            continue;
        }
        let category = get_general_category(character);
        if is_letter(category) || is_mark(category) {
            result.push(character.to_lowercase().next().unwrap_or(character));
        } else if is_punctuation(category)
            || character.is_whitespace()
            || category == Category::DecimalNumber
        {
            if result.last().is_some_and(|&last| last != ' ') {
                result.push(' ');
            }
        } else {
            result.push(character);
        }
    }
    result
}

fn is_letter(category: Category) -> bool {
    matches!(
        category,
        Category::UppercaseLetter
            | Category::LowercaseLetter
            | Category::TitlecaseLetter
            | Category::ModifierLetter
            | Category::OtherLetter
    )
}

fn is_mark(category: Category) -> bool {
    matches!(
        category,
        Category::NonspacingMark | Category::SpacingMark | Category::EnclosingMark
    )
}

fn is_punctuation(category: Category) -> bool {
    matches!(
        category,
        Category::ConnectorPunctuation
            | Category::DashPunctuation
            | Category::OpenPunctuation
            | Category::ClosePunctuation
            | Category::InitialPunctuation
            | Category::FinalPunctuation
            | Category::OtherPunctuation
    )
}

impl Profile {
    /// The geometric mean of the probabilities of the tokens.
    #[allow(clippy::cast_precision_loss)]
    fn score(&self, tokens: &[char]) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        let unigram_room = self.total as f64 + 0.1 * (self.single.keys.len() + 1) as f64;
        let mut log_sum = 0.0;
        let mut previous = 0;
        let mut pair = 0;
        for (index, &token) in tokens.iter().enumerate() {
            let character = u64::from(u32::from(token));
            // Smoothing and backing off avoid zero scores for words the corpus does not have,
            // while keeping the evidence from wrong decodings.
            let mut probability = (f64::from(self.single.get(character)) + 0.1) / unigram_room;
            if index >= 1 {
                let occurrences = self.pairs.get((previous << SHIFT) | character);
                probability = (f64::from(occurrences) + 4.0 * probability)
                    / (f64::from(self.single.get(previous)) + 4.0);
            }
            if index >= 2 {
                let occurrences = self.triples.get((pair << SHIFT) | character);
                probability = (f64::from(occurrences) + 4.0 * probability)
                    / (f64::from(self.pairs.get(pair)) + 4.0);
            }
            log_sum += probability.ln();
            pair = (previous << SHIFT) | character;
            previous = character;
        }
        (log_sum / tokens.len() as f64).exp()
    }
}

/// How certain it is that text is UTF-8, out of 100.
fn utf8_confidence(bytes: &[u8]) -> i32 {
    let has_bom = bytes.starts_with(b"\xEF\xBB\xBF");
    let mut valid = 0_usize;
    let mut invalid = 0_usize;
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte & 0x80 != 0 {
            let mut trail = if byte & 0xE0 == 0xC0 {
                1
            } else if byte & 0xF0 == 0xE0 {
                2
            } else if byte & 0xF8 == 0xF0 {
                3
            } else {
                invalid += 1;
                index += 1;
                continue;
            };
            loop {
                index += 1;
                let Some(&next) = bytes.get(index) else {
                    break;
                };
                if next & 0xC0 != 0x80 {
                    invalid += 1;
                    break;
                }
                trail -= 1;
                if trail == 0 {
                    valid += 1;
                    break;
                }
            }
        }
        index += 1;
    }
    if valid == 0 && invalid == 0 {
        // Plain ASCII says nothing.
        return 15;
    }
    confidence_from_counts(has_bom, valid, invalid)
}

fn confidence_from_counts(has_bom: bool, valid: usize, invalid: usize) -> i32 {
    if has_bom && invalid == 0 {
        100
    } else if has_bom && valid > invalid * 10 {
        80
    } else if valid > 3 && invalid == 0 {
        100
    } else if valid > 0 && invalid == 0 {
        80
    } else if valid > invalid * 10 {
        25
    } else {
        0
    }
}

/// How certain it is that text is UTF-16, out of 100, from its first characters.
fn utf16_confidence(bytes: &[u8], big_endian: bool) -> i32 {
    let checked = &bytes[..bytes.len().min(30)];
    let mut confidence = 10;
    for (index, unit) in checked.chunks_exact(2).enumerate() {
        let unit = if big_endian {
            u16::from_be_bytes([unit[0], unit[1]])
        } else {
            u16::from_le_bytes([unit[0], unit[1]])
        };
        if index == 0 && unit == 0xFEFF {
            // The same bytes also start UTF-32 in the same byte order.
            let is_utf32_bom = !big_endian && bytes.len() >= 4 && bytes[2] == 0 && bytes[3] == 0;
            return if is_utf32_bom { 0 } else { 100 };
        }
        if unit == 0 {
            confidence -= 10;
        } else if (0x20..=0xFF).contains(&unit) || unit == 0x0A {
            confidence += 10;
        }
        confidence = confidence.clamp(0, 100);
        if confidence == 0 || confidence == 100 {
            break;
        }
    }
    if checked.len() < 4 && confidence < 100 {
        0
    } else {
        confidence
    }
}

/// How certain it is that text is UTF-32, out of 100.
fn utf32_confidence(bytes: &[u8], big_endian: bool) -> i32 {
    let value = |unit: &[u8]| {
        let unit = [unit[0], unit[1], unit[2], unit[3]];
        if big_endian {
            u32::from_be_bytes(unit)
        } else {
            u32::from_le_bytes(unit)
        }
    };
    let mut units = bytes.chunks_exact(4).peekable();
    let Some(&first) = units.peek() else {
        return 0;
    };
    let has_bom = value(first) == 0xFEFF;
    if has_bom {
        units.next();
    }
    let mut valid = 0;
    let mut invalid = 0;
    for unit in units {
        let unit = value(unit);
        if unit >= 0x0010_FFFF || (0xD800..=0xDFFF).contains(&unit) {
            invalid += 1;
        } else {
            valid += 1;
        }
    }
    confidence_from_counts(has_bom, valid, invalid)
}

/// Recognizes Unicode by itself, when that is the best guess and the bytes are valid in it.
fn detect_unicode(bytes: &[u8]) -> Option<(&'static str, f64)> {
    let candidates = [
        ("utf8", utf8_confidence(bytes)),
        ("utf16be", utf16_confidence(bytes, true)),
        ("utf16", utf16_confidence(bytes, false)),
        ("utf32be", utf32_confidence(bytes, true)),
        ("utf32", utf32_confidence(bytes, false)),
    ];
    let mut best = candidates[0];
    for candidate in candidates {
        if candidate.1 > best.1 {
            best = candidate;
        }
    }
    if best.1 < MINIMUM_UNICODE_CONFIDENCE {
        return None;
    }
    let codepage = by_name(best.0)?;
    codepage
        .to_utf8(bytes)
        .ok()
        .map(|_| (codepage.name(), f64::from(best.1) / 100.0))
}

/// The number of multi-byte sequences in strictly valid UTF-8, and 0 for anything else.
fn utf8_sequences(bytes: &[u8]) -> usize {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.chars().filter(|c| !c.is_ascii()).count(),
        Err(_) => 0,
    }
}

/// How common a code page is: the lower, the more common.
fn rank(name: &str) -> i32 {
    if name.starts_with("utf") {
        0
    } else if name.starts_with("iso-8859") || name.starts_with("cp125") {
        1
    } else if matches!(
        name,
        "cp852" | "cp866" | "cp437" | "cp850" | "cp855" | "cp862"
    ) {
        2
    } else if name.starts_with("cp") || name.starts_with("mac-") || name.starts_with("apple-") {
        3
    } else if matches!(name, "mazovia" | "fido-mazovia") {
        4
    } else {
        5
    }
}

fn sort_detections(results: &mut [Detection]) {
    let descending = |a: f64, b: f64| b.partial_cmp(&a).unwrap_or(Ordering::Equal);
    results.sort_by(|a, b| {
        descending(a.confidence, b.confidence)
            .then_with(|| descending(a.score, b.score))
            .then_with(|| descending(a.language_score, b.language_score))
            .then_with(|| a.rank.cmp(&b.rank))
            .then_with(|| a.codepage.cmp(b.codepage))
            .then_with(|| a.language.cmp(b.language))
    });
}

/// Turns scores into probabilities, with the same prior for each pair of a decoding and a
/// language: P ~ score^(tokens / divisor). `bonus` adds log odds to some decodings.
#[allow(clippy::cast_precision_loss)]
fn assign_confidence(
    results: &mut [Detection],
    token_count: usize,
    bonus: &HashMap<i32, f64>,
    rarity_penalty: f64,
) {
    let mut joint: BTreeMap<(i32, &'static str), f64> = BTreeMap::new();
    let mut best = f64::NEG_INFINITY;
    for result in results.iter() {
        if result.score <= 0.0 {
            continue;
        }
        let mut logit = token_count as f64 * result.score.ln() / EVIDENCE_DIVISOR
            - rarity_penalty * f64::from((result.rank - 1).max(0));
        if let Some(extra) = bonus.get(&result.decoding) {
            logit += extra;
        }
        joint
            .entry((result.decoding, result.language))
            .and_modify(|entry| *entry = entry.max(logit))
            .or_insert(logit);
        best = best.max(logit);
    }
    let mut decodings: BTreeMap<i32, f64> = BTreeMap::new();
    let mut languages: BTreeMap<&'static str, f64> = BTreeMap::new();
    let mut total = 0.0;
    for (&(decoding, language), logit) in &joint {
        let weight = (logit - best).exp();
        *decodings.entry(decoding).or_default() += weight;
        *languages.entry(language).or_default() += weight;
        total += weight;
    }
    for result in results.iter_mut() {
        result.confidence = 0.0;
        result.language_confidence = 0.0;
        if result.score <= 0.0 || total == 0.0 {
            continue;
        }
        result.confidence = decodings[&result.decoding] / total;
        result.language_confidence = languages[result.language] / total;
    }
}

/// What can be asked of a unit of text when sampling it: a byte or a character.
trait SampleUnit: Copy + Eq {
    const SPACE: Self;
    const NEW_LINE: Self;
    const TAB: Self;
    const RETURN: Self;
    fn is_high(self) -> bool;
}

impl SampleUnit for u8 {
    const SPACE: Self = b' ';
    const NEW_LINE: Self = b'\n';
    const TAB: Self = b'\t';
    const RETURN: Self = b'\r';
    fn is_high(self) -> bool {
        self >= 0x80
    }
}

impl SampleUnit for char {
    const SPACE: Self = ' ';
    const NEW_LINE: Self = '\n';
    const TAB: Self = '\t';
    const RETURN: Self = '\r';
    fn is_high(self) -> bool {
        !self.is_ascii()
    }
}

/// The beginning, middle and end of a large text, to bound the time spent on it.
fn model_sample<T: SampleUnit>(input: &[T]) -> Vec<T> {
    if input.len() <= SAMPLE_PART * 3 {
        return input.to_vec();
    }
    let middle = (input.len() - SAMPLE_PART) / 2;
    let mut sample = input[..SAMPLE_PART].to_vec();
    sample.push(T::SPACE);
    sample.extend_from_slice(&input[middle..middle + SAMPLE_PART]);
    sample.push(T::SPACE);
    sample.extend_from_slice(&input[input.len() - SAMPLE_PART..]);
    sample
}

/// What ASCII surrounds the non-ASCII units, extended to whole words within the line. ASCII is
/// the same in every code page, so only this tells them apart. In source code it may be a few
/// comments that other samples would miss, and the code around them would mislead the models.
fn non_ascii_sample<T: SampleUnit>(input: &[T]) -> Vec<T> {
    const CONTEXT: usize = 16;
    let is_space = |unit: T| unit == T::SPACE || unit == T::TAB || unit == T::RETURN;
    let mut sample = Vec::new();
    let mut covered = 0;
    let mut index = 0;
    while index < input.len() && sample.len() < SAMPLE_PART * 3 {
        if !input[index].is_high() {
            index += 1;
            continue;
        }
        let mut from = index;
        while from > covered
            && input[from - 1] != T::NEW_LINE
            && index - from < 64
            && (index - from < CONTEXT || !is_space(input[from - 1]))
        {
            from -= 1;
        }
        // Past the context it ends at white space, never inside a multi-byte character.
        let mut to = index;
        while to < input.len()
            && input[to] != T::NEW_LINE
            && (to - index < CONTEXT || !is_space(input[to]))
        {
            to += 1;
        }
        sample.extend_from_slice(&input[from..to]);
        sample.push(T::SPACE);
        covered = to;
        index = to + 1;
    }
    if sample.is_empty() {
        model_sample(input)
    } else {
        sample
    }
}

/// A sample decoded in one code page.
struct DecodedSample {
    /// The characters kept; fewer than the bytes when some have no meaning in the code page.
    characters: usize,
    tokens: Vec<char>,
    token_count: usize,
    decoding: Option<i32>,
    odd_case: usize,
    text: String,
}

impl DecodedSample {
    fn new(text: String) -> Self {
        let mut tokens = tokens(&text);
        let token_count = tokens.len();
        // Only punctuation, digits or spaces: one word boundary, not an impossible text.
        if tokens.is_empty() && !text.is_empty() {
            tokens.push(' ');
        }
        let mut odd_case = 0;
        let mut was_lower = false;
        for character in text.chars() {
            let category = get_general_category(character);
            odd_case += usize::from(was_lower && category == Category::UppercaseLetter);
            was_lower = category == Category::LowercaseLetter;
        }
        Self {
            characters: text.chars().count(),
            tokens,
            token_count,
            decoding: None,
            odd_case,
            text,
        }
    }
}

/// Finds code pages, with the models loaded. They take some memory, so keep it only for as long
/// as it is needed.
pub struct Detector {
    profiles: Vec<Profile>,
}

impl Default for Detector {
    fn default() -> Self {
        Self::new()
    }
}

impl Detector {
    /// Unpacks the models of all languages.
    #[must_use]
    pub fn new() -> Self {
        Self {
            profiles: LANGUAGES.iter().map(load_profile).collect(),
        }
    }

    /// Ranks the candidates for the code page of a text, the most likely first. Empty input
    /// gives none.
    #[must_use]
    pub fn detect(&self, bytes: &[u8]) -> Vec<Detection> {
        if bytes.is_empty() {
            return Vec::new();
        }
        let has_bom = bytes.starts_with(b"\xEF\xBB\xBF");
        let sequences = if has_bom { 0 } else { utf8_sequences(bytes) };
        if sequences == 0
            && let Some((codepage, score)) = detect_unicode(bytes)
        {
            return self.detect_unicode_language(bytes, codepage, score);
        }
        self.detect_code_page(bytes, sequences)
    }

    /// The encoding is known to be Unicode, so only the language is uncertain.
    fn detect_unicode_language(
        &self,
        bytes: &[u8],
        codepage: &'static str,
        score: f64,
    ) -> Vec<Detection> {
        let alone = Detection {
            codepage,
            score,
            rank: 0,
            language: "",
            language_score: 0.0,
            confidence: score,
            language_confidence: 0.0,
            decoding: -1,
        };
        let text = by_name(codepage).map_or_else(String::new, |found| found.without_invalid(bytes));
        let characters: Vec<char> = text.chars().collect();
        let sample: String = non_ascii_sample(&characters).into_iter().collect();
        let sample = DecodedSample::new(sample);
        let mut results: Vec<Detection> = self
            .profiles
            .iter()
            .map(|profile| {
                let language_score = if sample.text.is_empty() {
                    0.0
                } else {
                    profile.score(&sample.tokens)
                };
                Detection {
                    language: profile.language.iso,
                    language_score,
                    score: language_score,
                    ..alone.clone()
                }
            })
            .collect();
        assign_confidence(&mut results, sample.token_count, &HashMap::new(), 0.0);
        for result in &mut results {
            result.score = score;
            result.confidence = if result.language_score > 0.0 {
                score
            } else {
                0.0
            };
        }
        sort_detections(&mut results);
        if results
            .first()
            .is_none_or(|first| first.language_score == 0.0)
        {
            return vec![alone];
        }
        results
    }

    /// Compares each code page of each language, and UTF-8 if the bytes are valid in it.
    #[allow(clippy::cast_precision_loss)]
    fn detect_code_page(&self, bytes: &[u8], utf8_sequences: usize) -> Vec<Detection> {
        let sample = non_ascii_sample(bytes);
        let mut decoded: HashMap<&'static str, DecodedSample> = HashMap::new();
        let mut decodings: HashMap<String, i32> = HashMap::new();
        let mut bonus: HashMap<i32, f64> = HashMap::new();
        let mut results = Vec::new();
        let mut token_count = 0;
        for profile in &self.profiles {
            // The same text scores the same, whichever code page gave it.
            let mut scores: HashMap<i32, f64> = HashMap::new();
            let extra: &[&'static str] = if utf8_sequences > 0 { &["utf8"] } else { &[] };
            let mut seen: HashSet<&'static str> = HashSet::new();
            for &name in profile.language.charsets.iter().chain(extra) {
                if !seen.insert(name) {
                    continue;
                }
                let Some(codepage) = by_name(name) else {
                    continue;
                };
                let name = codepage.name();
                let this = decoded.entry(name).or_insert_with(|| {
                    let this = DecodedSample::new(codepage.without_invalid(&sample));
                    token_count = token_count.max(this.token_count);
                    this
                });
                // A byte without a meaning cannot give lossless text.
                if name != "utf8" && this.characters != sample.len() {
                    continue;
                }
                let decoding = *this.decoding.get_or_insert_with(|| {
                    let next = i32::try_from(decodings.len()).unwrap_or(i32::MAX);
                    *decodings.entry(this.text.clone()).or_insert_with(|| {
                        let utf8_bonus = if name == "utf8" {
                            utf8_sequences as f64 * UTF8_EVIDENCE
                        } else {
                            0.0
                        };
                        bonus.insert(next, utf8_bonus - CASE_PENALTY * this.odd_case as f64);
                        next
                    })
                });
                let score = *scores.entry(decoding).or_insert_with(|| {
                    if this.text.is_empty() {
                        0.0
                    } else {
                        profile.score(&this.tokens)
                    }
                });
                results.push(Detection {
                    codepage: name,
                    score,
                    rank: rank(name),
                    language: profile.language.iso,
                    language_score: score,
                    confidence: 0.0,
                    language_confidence: 0.0,
                    decoding,
                });
            }
        }
        // A likely candidate must also keep each byte outside the sample. Those that fail go,
        // and the probabilities are computed again without them.
        let mut verified: HashSet<&'static str> = HashSet::from(["utf8"]);
        loop {
            assign_confidence(&mut results, token_count, &bonus, RARITY_PENALTY);
            sort_detections(&mut results);
            let mut covered = 0.0;
            let mut counted: HashSet<i32> = HashSet::new();
            let mut failed = None;
            for result in &results {
                if covered >= 1.0 - 1e-6 {
                    break;
                }
                if counted.insert(result.decoding) {
                    covered += result.confidence;
                }
                if !verified.insert(result.codepage) {
                    continue;
                }
                if by_name(result.codepage).is_some_and(|found| found.to_utf8(bytes).is_err()) {
                    failed = Some(result.codepage);
                    break;
                }
            }
            let Some(failed) = failed else {
                return results;
            };
            results.retain(|result| result.codepage != failed);
        }
    }

    /// The alternatives worth considering: the fewest different decodings whose confidence adds
    /// up to `threshold`, each with its likeliest language and code page. One means a confident
    /// winner; with more, the caller should not decide alone. Expects what [`Self::detect`] gives.
    #[must_use]
    pub fn plausible(results: &[Detection], threshold: f64) -> Vec<Detection> {
        let mut chosen = Vec::new();
        let mut seen = HashSet::new();
        let mut covered = 0.0;
        for result in results {
            if covered >= threshold {
                break;
            }
            if !seen.insert(result.decoding) {
                continue;
            }
            chosen.push(result.clone());
            covered += result.confidence;
        }
        chosen
    }
}
