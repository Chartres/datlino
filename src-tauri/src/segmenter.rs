//! Czech/Slovak-aware sentence segmentation.
//!
//! Splits a `.md` or `.txt` document into typing-ready sentences while
//! handling the things that trip up generic English segmenters:
//!
//!   * Czech/Slovak abbreviations that contain a period but never end a
//!     sentence (`např.`, `tzv.`, `atd.`, `tj.`, `resp.`, `apod.`, `napr.`,
//!     `atď.`, academic titles, units, …).
//!   * Roman numerals next to monarch/pope names — `Jindřich VIII.`,
//!     `Karel IV.`, `Pius XII.` — which look like sentence ends but aren't.
//!   * Numbered list markers — `1. První bod` should produce one chunk
//!     `První bod`, not two halves around the period.
//!   * Decimal numbers — `3.14`, `1,5` (CZ comma-decimal is also fine).
//!   * Markdown headers (`#`, `##`, …), bullet lists (`-`, `*`, `+`),
//!     numbered lists (`1.`, `1)`), and blockquotes (`>`).
//!   * Diacritics: input is NFC-normalised; segmenter never strips accents.

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;
use unicode_normalization::UnicodeNormalization;

/// One typing-ready sentence carved out of a source document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentence {
    /// The trimmed sentence text. Diacritics intact, NFC-normalised.
    pub text: String,
    /// Byte offset into the *NFC-normalised* source where the sentence
    /// begins. Stored under the schema column `chunk.char_offset`.
    pub byte_offset: usize,
    /// The paragraph (or list item) the sentence belongs to. Used for
    /// provenance/recall when the typing UI wants to show context.
    pub context: String,
    /// Markdown section path ("Habsburkové > Stavovské povstání"). Empty
    /// string if the sentence is at document root before any header.
    pub section: String,
    /// True when this sentence *is* a header line (`#`, `##`, …). Heading
    /// chunks are useful for chapter navigation but typically skipped when
    /// building a typing session.
    pub is_heading: bool,
}

/// Lowercased word stems that carry a trailing period without ending a
/// sentence. Matched case-insensitively against the word immediately
/// preceding a `.`.
static ABBREVIATIONS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        // Czech everyday
        "např", "tzv", "atd", "atp", "apod", "aj", "tj", "tzn", "resp",
        "popř", "mj", "ev", "cca", "viz", "vč",
        // Slovak everyday
        "napr", "atď", "napríkl", "vrát", "popr", "tzn",
        // Reference / bibliographic
        "str", "č", "r", "kap", "obr", "tab", "pozn", "sv", "stol", "stor",
        "s", "ev", "vyd", "roč", "odst",
        // People / titles (CZ + SK)
        "p", "pí", "pp", "mr", "mrs", "ms",
        "doc", "prof", "ing", "mgr", "bc", "phdr", "mudr", "judr", "rndr",
        "dr", "ph", "csc", "drsc",
        // Units / quantities
        "max", "min", "kč", "eur", "tis", "mil", "mld",
        // Months sometimes abbreviated
        "led", "úno", "bře", "dub", "kvě", "čvn", "čvc", "srp", "zář", "říj",
        "lis", "pro",
    ]
    .into_iter()
    .collect()
});

/// Whole-string roman-numeral check (uppercase letters I V X L C D M only).
static ROMAN_NUMERAL: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[IVXLCDM]+$").unwrap());

/// Markdown line prefix patterns we strip before scanning prose. Each
/// pattern captures the markup so we can advance the byte offset accordingly.
/// Handles ASCII markdown bullets (`- * +`), Unicode bullets often pasted
/// from Word / PDFs / OCR output (`• ● ◦ ‣ ▪ ○ □ ◇ ◆ – —`), and numbered
/// markers (`1.`, `1)`, `a)`).
static LIST_MARKER: Lazy<Regex> = Lazy::new(|| {
    // ASCII markers require whitespace after them so we don't eat a `-` in
    // the middle of a word. Unicode bullets tolerate missing whitespace
    // because OCR often glues them onto the first word.
    Regex::new(
        r"^(?:[\-\*\+]\s+|[\u{2022}\u{25CF}\u{25E6}\u{2023}\u{25AA}\u{25CB}\u{25A1}\u{25C7}\u{25C6}\u{2013}\u{2014}]\s*|\d+[\.\)]\s+|[a-zA-Z\u{00E1}-\u{017E}]\)\s+)",
    )
    .unwrap()
});

static HEADER_MARKER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(#{1,6}\s+)").unwrap());

static BLOCKQUOTE_MARKER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(>+\s*)").unwrap());

/// Public entry point: split a document into typing-ready sentences.
pub fn segment(source: &str) -> Vec<Sentence> {
    let normalised: String = source.nfc().collect();
    let mut out = Vec::new();
    // Header stack: index = depth-1, value = header title at that depth.
    let mut section_stack: Vec<String> = Vec::new();

    for paragraph in iter_paragraphs(&normalised) {
        for line in iter_lines(&paragraph) {
            let mut local_offset = line.byte_offset;
            let mut body = line.text.as_str();
            let mut is_heading = false;

            if let Some(m) = HEADER_MARKER.find(body) {
                // Depth = number of leading `#` characters.
                let depth = body[..m.end()].chars().filter(|c| *c == '#').count();
                local_offset += m.end();
                body = &body[m.end()..];
                let title = body.trim_end().to_string();
                update_section_stack(&mut section_stack, depth, title);
                is_heading = true;
            } else if let Some(m) = LIST_MARKER.find(body) {
                local_offset += m.end();
                body = &body[m.end()..];
            } else if let Some(m) = BLOCKQUOTE_MARKER.find(body) {
                local_offset += m.end();
                body = &body[m.end()..];
            }

            let body = body.trim_end();
            if body.is_empty() {
                continue;
            }

            let section = section_path(&section_stack);
            let before = out.len();
            scan_prose(body, local_offset, &paragraph.text, &mut out);
            // Stamp section + heading flag on every sentence this line produced.
            for s in &mut out[before..] {
                s.section = section.clone();
                s.is_heading = is_heading;
            }
        }
    }

    out
}

fn update_section_stack(stack: &mut Vec<String>, depth: usize, title: String) {
    let depth = depth.max(1);
    // Any deeper levels become stale; truncate.
    stack.truncate(depth - 1);
    // Pad with empties so index-by-depth stays consistent when a deeper
    // header appears before the shallower one (rare but possible).
    while stack.len() < depth - 1 {
        stack.push(String::new());
    }
    stack.push(title);
}

fn section_path(stack: &[String]) -> String {
    stack
        .iter()
        .filter(|s| !s.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" > ")
}

#[derive(Debug)]
struct Paragraph {
    text: String,
    byte_offset: usize,
}

#[derive(Debug)]
struct LineInParagraph {
    text: String,
    byte_offset: usize,
}

/// Iterate paragraphs (groups of non-blank lines separated by blank lines).
fn iter_paragraphs(source: &str) -> Vec<Paragraph> {
    let mut paragraphs = Vec::new();
    let mut buf = String::new();
    let mut start: Option<usize> = None;
    let mut cursor = 0usize;

    for line in source.split_inclusive('\n') {
        let line_start = cursor;
        cursor += line.len();
        let trimmed = line.trim_end_matches(['\r', '\n']);

        if trimmed.trim().is_empty() {
            if let Some(s) = start.take() {
                paragraphs.push(Paragraph {
                    text: std::mem::take(&mut buf).trim_end().to_string(),
                    byte_offset: s,
                });
            }
            continue;
        }
        if start.is_none() {
            start = Some(line_start);
        }
        buf.push_str(line);
    }
    if let Some(s) = start.take() {
        paragraphs.push(Paragraph {
            text: buf.trim_end().to_string(),
            byte_offset: s,
        });
    }
    paragraphs
}

/// Iterate non-empty lines inside a paragraph, with byte offsets relative
/// to the original source.
fn iter_lines(paragraph: &Paragraph) -> Vec<LineInParagraph> {
    let mut lines = Vec::new();
    let mut cursor = 0usize;
    for line in paragraph.text.split_inclusive('\n') {
        let line_start = paragraph.byte_offset + cursor;
        cursor += line.len();
        let stripped = line.trim_end_matches(['\r', '\n']).to_string();
        if stripped.trim().is_empty() {
            continue;
        }
        lines.push(LineInParagraph {
            text: stripped,
            byte_offset: line_start,
        });
    }
    lines
}

/// Walk a stripped prose line and emit one or more sentences. Boundaries
/// are detected at `.`, `!`, `?`, `…` followed by whitespace + an
/// uppercase letter (or end-of-line), with abbreviation/decimal/roman
/// guards applied.
fn scan_prose(body: &str, base_offset: usize, context: &str, out: &mut Vec<Sentence>) {
    let bytes = body.as_bytes();
    let len = bytes.len();
    let mut sentence_start = 0usize;
    let mut i = 0usize;

    while i < len {
        // Skip continuation bytes of multi-byte chars; we only inspect
        // terminals at char boundaries.
        if !body.is_char_boundary(i) {
            i += 1;
            continue;
        }
        let c = bytes[i] as char;
        let is_terminal = matches!(c, '.' | '!' | '?');

        // Fast-path ASCII terminal handling first.
        if is_terminal {
            // Collapse runs like "?!", "!!!", "..." into one boundary at the
            // last terminal char.
            let mut end = i;
            while end + 1 < len
                && matches!(bytes[end + 1] as char, '.' | '!' | '?')
            {
                end += 1;
            }

            if should_break(body, sentence_start, end) {
                push_sentence(body, sentence_start, end + 1, base_offset, context, out);
                // Advance past the terminal run + following whitespace.
                let mut j = end + 1;
                while j < len && (bytes[j] as char).is_ascii_whitespace() {
                    j += 1;
                }
                sentence_start = j;
                i = j;
                continue;
            } else {
                i = end + 1;
                continue;
            }
        }

        // Unicode ellipsis (U+2026 — UTF-8: 0xE2 0x80 0xA6). Compare bytes
        // directly so we never slice across a char boundary.
        if i + 3 <= len
            && bytes[i] == 0xE2
            && bytes[i + 1] == 0x80
            && bytes[i + 2] == 0xA6
        {
            let end = i + 2; // index of last byte of "…"
            if should_break(body, sentence_start, end) {
                push_sentence(body, sentence_start, end + 1, base_offset, context, out);
                let mut j = end + 1;
                while j < len && (bytes[j] as char).is_ascii_whitespace() {
                    j += 1;
                }
                sentence_start = j;
                i = j;
                continue;
            }
        }

        i += 1;
    }

    // Trailing fragment with no terminal punctuation — still emit it (a
    // markdown header line, a list item without a period, …).
    if sentence_start < len {
        push_sentence(body, sentence_start, len, base_offset, context, out);
    }
}

/// Decide whether the terminal run ending at byte index `end` (inclusive)
/// is a true sentence boundary.
fn should_break(body: &str, sentence_start: usize, end: usize) -> bool {
    let bytes = body.as_bytes();
    let len = bytes.len();
    let last = bytes[end] as char;

    // For `?` and `!` we always break (subject to a sane look-ahead).
    if last == '?' || last == '!' {
        return next_starts_new_sentence(body, end + 1);
    }

    // For `.` and `…` we apply the abbreviation / decimal / roman guards.
    let preceding = preceding_word(&body[sentence_start..=end - if last == '.' { 1 } else { 0 }]);

    if last == '.' {
        // Decimal: digit . digit  (e.g. 3.14)
        if end > 0 && end + 1 < len {
            let prev = bytes[end - 1] as char;
            let next = bytes[end + 1] as char;
            if prev.is_ascii_digit() && next.is_ascii_digit() {
                return false;
            }
        }
        // Pure abbreviation (case-insensitive ASCII fold of the preceding
        // word's stem; we also lower-case unicode for CZ/SK letters).
        let stem = strip_diacritics_ascii(&preceding.to_lowercase());
        if ABBREVIATIONS.contains(stem.as_str())
            || ABBREVIATIONS.contains(preceding.to_lowercase().as_str())
        {
            return false;
        }
        // Roman numeral immediately before the period (e.g. "VIII.").
        if !preceding.is_empty() && ROMAN_NUMERAL.is_match(&preceding) {
            return false;
        }
    }

    next_starts_new_sentence(body, end + 1)
}

/// Look ahead: skip whitespace, then require uppercase letter, digit, or
/// end-of-string. Lowercase next char ⇒ probably mid-sentence.
fn next_starts_new_sentence(body: &str, from: usize) -> bool {
    let mut i = from;
    let bytes = body.as_bytes();
    let len = bytes.len();
    while i < len {
        let c = bytes[i] as char;
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        // Multi-byte char — decode it.
        if !c.is_ascii() {
            let rest = &body[i..];
            if let Some(ch) = rest.chars().next() {
                // Skip Unicode opening quotes/brackets (CZ uses „…")
                if matches!(ch, '„' | '«' | '‹' | '»' | '"' | '\u{2018}' | '\u{2019}') {
                    i += ch.len_utf8();
                    continue;
                }
                return ch.is_uppercase();
            }
            return true;
        }
        // ASCII: opening quotes / parens count as start of next sentence.
        if matches!(c, '"' | '\'' | '(' | '[') {
            i += 1;
            continue;
        }
        return c.is_ascii_uppercase() || c.is_ascii_digit();
    }
    true // end of body
}

/// Take the trailing word before the terminal run.
fn preceding_word(slice: &str) -> String {
    let mut buf = String::new();
    for c in slice.chars().rev() {
        if c.is_alphabetic() {
            buf.push(c);
        } else {
            break;
        }
    }
    buf.chars().rev().collect()
}

/// Cheap ASCII fold for matching against the abbreviation table — turns
/// `např` into `napr`, `atď` into `atd`, etc. We keep the original-case
/// match too, in case the table already contains the diacritic form.
fn strip_diacritics_ascii(s: &str) -> String {
    s.nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .map(|c| match c {
            'ł' => 'l',
            'Ł' => 'L',
            other => other,
        })
        .collect()
}

fn push_sentence(
    body: &str,
    start: usize,
    end: usize,
    base_offset: usize,
    context: &str,
    out: &mut Vec<Sentence>,
) {
    let raw = &body[start..end];
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return;
    }
    // Adjust the recorded offset for any leading whitespace we trimmed.
    let leading_ws = raw.len() - raw.trim_start().len();
    out.push(Sentence {
        text: trimmed.to_string(),
        byte_offset: base_offset + start + leading_ws,
        context: context.trim().to_string(),
        section: String::new(),
        is_heading: false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(s: &[Sentence]) -> Vec<&str> {
        s.iter().map(|x| x.text.as_str()).collect()
    }

    #[test]
    fn empty_input_yields_nothing() {
        assert!(segment("").is_empty());
        assert!(segment("   \n\n  \n").is_empty());
    }

    #[test]
    fn single_sentence() {
        let s = segment("Habsburkové vládli v Čechách.");
        assert_eq!(texts(&s), vec!["Habsburkové vládli v Čechách."]);
        assert_eq!(s[0].byte_offset, 0);
    }

    #[test]
    fn two_sentences_split_on_period() {
        let s = segment("První věta. Druhá věta.");
        assert_eq!(texts(&s), vec!["První věta.", "Druhá věta."]);
    }

    #[test]
    fn abbreviation_napr_does_not_split() {
        let s = segment("Napsal řadu spisů, např. dějepisné práce a básně.");
        assert_eq!(
            texts(&s),
            vec!["Napsal řadu spisů, např. dějepisné práce a básně."]
        );
    }

    #[test]
    fn abbreviation_tzv_does_not_split() {
        let s = segment("Setkal se s tzv. otázkou rolnické emancipace.");
        assert_eq!(
            texts(&s),
            vec!["Setkal se s tzv. otázkou rolnické emancipace."]
        );
    }

    #[test]
    fn abbreviation_atd_followed_by_real_sentence() {
        let s = segment("Přijeli šlechtici, biskup atd. Pak dorazil i král.");
        assert_eq!(
            texts(&s),
            vec!["Přijeli šlechtici, biskup atd. Pak dorazil i král."],
            "atd. before capital is still an abbreviation, not a boundary"
        );
    }

    #[test]
    fn slovak_abbreviations() {
        let s = segment("Boli tam vojaci, kňazi atď. Potom prišiel kráľ.");
        assert_eq!(texts(&s).len(), 1);
        let s2 = segment("Mnohé štúdie, napr. najnovšie z Bratislavy, to potvrdzujú.");
        assert_eq!(texts(&s2).len(), 1);
    }

    #[test]
    fn numbered_list_items_are_separate_sentences() {
        let src = "1. První bod\n2. Druhý bod\n3. Třetí bod";
        let s = segment(src);
        assert_eq!(texts(&s), vec!["První bod", "Druhý bod", "Třetí bod"]);
    }

    #[test]
    fn bullet_list_items_are_separate_sentences() {
        let src = "- První bod\n- Druhý bod\n* Třetí bod\n+ Čtvrtý bod";
        let s = segment(src);
        assert_eq!(
            texts(&s),
            vec!["První bod", "Druhý bod", "Třetí bod", "Čtvrtý bod"]
        );
    }

    #[test]
    fn unicode_bullet_markers_are_stripped() {
        // Common bullets students paste from Word / Notion / PDF exports.
        let src = "• První bod\n● Druhý bod\n◦ Třetí bod\n– Čtvrtý bod";
        let s = segment(src);
        assert_eq!(
            texts(&s),
            vec!["První bod", "Druhý bod", "Třetí bod", "Čtvrtý bod"]
        );
    }

    #[test]
    fn unicode_bullet_without_trailing_space_is_stripped() {
        // Some OCR output glues the bullet to the first word.
        let src = "●Velká hospodářská krize (1929)";
        let s = segment(src);
        assert_eq!(texts(&s), vec!["Velká hospodářská krize (1929)"]);
    }

    #[test]
    fn numbered_list_with_multi_sentence_items() {
        let src = "1. První věta. Druhá věta.\n2. Třetí věta.";
        let s = segment(src);
        assert_eq!(
            texts(&s),
            vec!["První věta.", "Druhá věta.", "Třetí věta."]
        );
    }

    #[test]
    fn paragraphs_separated_by_blank_line() {
        let src = "První odstavec má jen jednu větu.\n\nDruhý odstavec má dvě věty. Tady je ta druhá.";
        let s = segment(src);
        assert_eq!(
            texts(&s),
            vec![
                "První odstavec má jen jednu větu.",
                "Druhý odstavec má dvě věty.",
                "Tady je ta druhá."
            ]
        );
    }

    #[test]
    fn decimal_numbers_do_not_split() {
        let s = segment("Hodnota π je přibližně 3.14 a často se zaokrouhluje.");
        assert_eq!(texts(&s).len(), 1);
    }

    #[test]
    fn year_at_end_of_sentence_does_split() {
        let s = segment("Bitva proběhla roku 1620. Následovala porážka stavů.");
        assert_eq!(
            texts(&s),
            vec!["Bitva proběhla roku 1620.", "Následovala porážka stavů."]
        );
    }

    #[test]
    fn roman_numeral_after_monarch_does_not_split() {
        let s = segment("Karel IV. byl římským císařem. Vládl ve 14. století.");
        assert_eq!(
            texts(&s),
            vec![
                "Karel IV. byl římským císařem.",
                "Vládl ve 14. století."
            ]
        );
    }

    #[test]
    fn questions_and_exclamations_split() {
        let s = segment("Co je to fotosyntéza? Je to proces. Zajímavé!");
        assert_eq!(
            texts(&s),
            vec!["Co je to fotosyntéza?", "Je to proces.", "Zajímavé!"]
        );
    }

    #[test]
    fn diacritics_are_preserved() {
        let s = segment("Píšeme čeština a slovenčina.");
        assert!(s[0].text.contains("čeština"));
        assert!(s[0].text.contains("slovenčina"));
    }

    #[test]
    fn header_becomes_a_sentence_without_period() {
        let s = segment("# Habsburkové\n\nVládli v Čechách.");
        assert_eq!(texts(&s), vec!["Habsburkové", "Vládli v Čechách."]);
    }

    #[test]
    fn headers_become_section_paths_on_following_sentences() {
        let src = "# Dějepis\n\nIntro věta.\n\n## Habsburkové\n\nVládli.\n\n## Lucemburkové\n\nTaké vládli.";
        let s = segment(src);
        let header = s.iter().find(|x| x.text == "Dějepis").unwrap();
        assert!(header.is_heading);
        let intro = s.iter().find(|x| x.text == "Intro věta.").unwrap();
        assert_eq!(intro.section, "Dějepis");
        assert!(!intro.is_heading);
        let hab = s.iter().find(|x| x.text == "Vládli.").unwrap();
        assert_eq!(hab.section, "Dějepis > Habsburkové");
        let luc = s.iter().find(|x| x.text == "Také vládli.").unwrap();
        assert_eq!(luc.section, "Dějepis > Lucemburkové");
    }

    #[test]
    fn blockquote_marker_is_stripped() {
        let s = segment("> Citát z učebnice.\n> Další věta.");
        assert_eq!(texts(&s), vec!["Citát z učebnice.", "Další věta."]);
    }

    #[test]
    fn byte_offsets_point_into_original_source() {
        let src = "První věta. Druhá věta.";
        let s = segment(src);
        assert_eq!(s[0].byte_offset, 0);
        // "Druhá" starts after "První věta. " — 12 ASCII bytes ('í' is 2 bytes).
        // "P-r-í" is 1+1+2=4 bytes; "rvní" is r-v-n-í = 1+1+1+2 = 5 bytes; "P"+"rvní"=6;
        // "P-r-v-n-í" then space + "věta." then space:
        // P(1) r(1) v(1) n(1) í(2) space(1) v(1) ě(2) t(1) a(1) .(1) space(1) = 14
        let expected = src.find("Druhá").unwrap();
        assert_eq!(s[1].byte_offset, expected);
    }

    #[test]
    fn context_is_the_paragraph() {
        let src = "První věta. Druhá věta.\n\nJiný odstavec.";
        let s = segment(src);
        assert_eq!(s[0].context, "První věta. Druhá věta.");
        assert_eq!(s[1].context, "První věta. Druhá věta.");
        assert_eq!(s[2].context, "Jiný odstavec.");
    }

    #[test]
    fn ellipsis_three_dots_collapsed() {
        let s = segment("Pak ticho... A přišel.");
        assert_eq!(texts(&s), vec!["Pak ticho...", "A přišel."]);
    }

    #[test]
    fn unicode_ellipsis_char() {
        let s = segment("Pak ticho… A přišel.");
        assert_eq!(texts(&s), vec!["Pak ticho…", "A přišel."]);
    }

    #[test]
    fn question_then_exclamation_kept_together() {
        let s = segment("Opravdu?! Jistě.");
        assert_eq!(texts(&s), vec!["Opravdu?!", "Jistě."]);
    }
}
