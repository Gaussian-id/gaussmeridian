use std::ops::Range;

use gaussmeridian_models::request::{Content, ContentPart, Message, Role};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MERIDIAN_ACTIVE_INSTRUCTION_V1_VERSION: &str = "meridian-active-instruction/v1";
pub const MERIDIAN_ACTIVE_INSTRUCTION_VERSION: &str = "meridian-active-instruction/v2";

const ENGLISH_DEMANDING_OPERATORS: &[&str] = &[
    "analyse",
    "analyze",
    "assess",
    "calculate",
    "choose",
    "compare",
    "compute",
    "critique",
    "debug",
    "decide",
    "demonstrate",
    "derive",
    "design",
    "determine",
    "diagnose",
    "evaluate",
    "implement",
    "infer",
    "investigate",
    "optimize",
    "optimise",
    "plan",
    "prove",
    "recommend",
    "solve",
    "synthesize",
    "synthesise",
    "verify",
];
const INDONESIAN_DEMANDING_OPERATORS: &[&str] = &[
    "analisis",
    "bandingkan",
    "buktikan",
    "debug",
    "diagnosis",
    "evaluasi",
    "hitung",
    "implementasikan",
    "investigasi",
    "kritik",
    "optimalkan",
    "rancang",
    "selesaikan",
    "sintesis",
    "tentukan",
    "turunkan",
    "verifikasi",
];
const ENGLISH_ROUTINE_OPERATORS: &[&str] = &[
    "classify",
    "explain",
    "extract",
    "rewrite",
    "summarise",
    "summarize",
    "translate",
];
const INDONESIAN_ROUTINE_OPERATORS: &[&str] = &[
    "ekstrak",
    "jelaskan",
    "klasifikasikan",
    "rangkum",
    "terjemahkan",
    "tulis ulang",
];
const FORMAL_CONCEPTS: &[&str] = &[
    "abelian",
    "algorithm",
    "algoritma",
    "bilangan",
    "bounded",
    "calculus",
    "combinatorics",
    "concurrent",
    "concurrency",
    "congruence",
    "constraint",
    "correctness",
    "derivative",
    "distributed",
    "eigenvalue",
    "field",
    "graf",
    "graph",
    "group",
    "grup",
    "ideal",
    "induction",
    "induksi",
    "indemnification",
    "integral",
    "invariant",
    "jurisdiction",
    "jurisdictional",
    "lapangan",
    "liability",
    "matrix",
    "matriks",
    "memory-ordering",
    "order",
    "probability",
    "proof",
    "race-free",
    "recurrence",
    "regulatory",
    "ring",
    "squared",
    "theorem",
    "topology",
    "variance",
];
const ENGLISH_LANGUAGE_MARKERS: &[&str] = &[
    "and",
    "are",
    "every",
    "first",
    "following",
    "instead",
    "is",
    "supplied",
    "that",
    "the",
    "this",
    "whether",
    "with",
];
const INDONESIAN_LANGUAGE_MARKERS: &[&str] = &[
    "adalah", "apakah", "bahwa", "berikut", "dengan", "dan", "dari", "ganjil", "jelaskan",
    "jumlah", "kuadrat", "pertama", "setiap", "tidak", "yang",
];
const NEGATION_PREFIXES: &[&str] = &[
    "do not ",
    "don't ",
    "never ",
    "no need to ",
    "without ",
    "jangan ",
    "tidak perlu ",
    "tak perlu ",
    "tanpa ",
];
const CONTRAST_BOUNDARIES: &[&str] = &[
    " instead",
    " but ",
    " however",
    " rather ",
    " tapi ",
    " namun ",
    " melainkan ",
];
const DATA_MARKERS: &[&str] = &[
    "data berikut",
    "following data",
    "following log",
    "following text",
    "kutipan berikut",
    "log berikut",
    "quoted text",
    "supplied data",
    "supplied log",
    "supplied text",
    "teks berikut",
    "this data",
    "this log",
    "this text",
];
const DATA_FOLLOW_ON_BOUNDARIES: &[&str] = &[
    ". afterwards ",
    ". finally ",
    ". however ",
    ". instead ",
    ". kemudian ",
    ". lalu ",
    ". next ",
    ". selanjutnya ",
    ". setelah itu ",
    ". subsequently ",
    ". then ",
    "; kemudian ",
    "; lalu ",
    "; then ",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstructionLanguageProfile {
    English,
    Indonesian,
    Mixed,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstructionSpanKind {
    Active,
    QuotedData,
    Negated,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstructionSpanEvidence {
    pub start: u32,
    pub end: u32,
    pub kind: InstructionSpanKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveInstructionEvidence {
    pub analysis_version: String,
    pub language_profile: InstructionLanguageProfile,
    pub normalized_input_sha256: String,
    pub normalized_input_bytes: u32,
    pub active_instruction_tokens: u32,
    pub quoted_data_tokens: u32,
    pub spans: Vec<InstructionSpanEvidence>,
    pub demanding_operators: Vec<String>,
    pub routine_operators: Vec<String>,
    pub formal_concepts: Vec<String>,
    pub suppressed_operators: Vec<String>,
}

impl ActiveInstructionEvidence {
    pub(crate) fn validate(&self) -> bool {
        if !matches!(
            self.analysis_version.as_str(),
            MERIDIAN_ACTIVE_INSTRUCTION_V1_VERSION | MERIDIAN_ACTIVE_INSTRUCTION_VERSION
        ) || !is_canonical_sha256(&self.normalized_input_sha256)
        {
            return false;
        }
        let mut previous_end = 0;
        for span in &self.spans {
            if span.start >= span.end
                || span.start < previous_end
                || span.end > self.normalized_input_bytes
            {
                return false;
            }
            previous_end = span.end;
        }
        [
            &self.demanding_operators,
            &self.routine_operators,
            &self.formal_concepts,
            &self.suppressed_operators,
        ]
        .into_iter()
        .all(|items| {
            items.iter().all(|item| !item.trim().is_empty())
                && items.windows(2).all(|pair| pair[0] < pair[1])
        })
    }
}

pub(crate) struct ActiveInstructionAnalysis {
    pub active_text: String,
    pub lexical_text: String,
    pub evidence: ActiveInstructionEvidence,
}

#[derive(Clone)]
struct MessageRegion {
    range: Range<usize>,
    active_role: bool,
}

pub(crate) fn analyze(messages: &[Message]) -> ActiveInstructionAnalysis {
    let (normalized, regions) = normalize_messages(messages);
    let mut quoted_data = Vec::new();
    let mut declared_data = Vec::new();
    let mut negated = Vec::new();

    for region in &regions {
        if !region.active_role {
            quoted_data.push(region.range.clone());
            continue;
        }
        let text = &normalized[region.range.clone()];
        declared_data.extend(
            declared_data_ranges(text)
                .into_iter()
                .map(|range| shift_range(range, region.range.start)),
        );
        quoted_data.extend(
            quoted_data_ranges(text)
                .into_iter()
                .map(|range| shift_range(range, region.range.start)),
        );
    }
    quoted_data = merge_ranges(quoted_data);
    declared_data = merge_ranges(declared_data);

    for region in regions.iter().filter(|region| region.active_role) {
        let text = &normalized[region.range.clone()];
        negated.extend(
            negated_ranges(text)
                .into_iter()
                .map(|range| shift_range(range, region.range.start)),
        );
    }
    negated = merge_ranges(negated);
    negated = complement_ranges(&negated, &quoted_data, &[]);

    let eligible_regions = regions
        .iter()
        .filter(|region| region.active_role)
        .map(|region| region.range.clone())
        .collect::<Vec<_>>();
    let lexical = complement_ranges(&eligible_regions, &declared_data, &negated);
    let lexical_text = joined_ranges(&normalized, &lexical);
    let active = complement_ranges(&eligible_regions, &quoted_data, &negated);
    let active_text = joined_ranges(&normalized, &active);
    let active_instruction_tokens = estimate_tokens(&active_text);
    let quoted_text = joined_ranges(&normalized, &quoted_data);
    let negated_text = joined_ranges(&normalized, &negated);
    let lower_active = active_text.to_lowercase();
    let lower_negated = negated_text.to_lowercase();

    let mut demanding_operators = matched_words(
        &lower_active,
        ENGLISH_DEMANDING_OPERATORS
            .iter()
            .chain(INDONESIAN_DEMANDING_OPERATORS),
    );
    let mut routine_operators = matched_words(
        &lower_active,
        ENGLISH_ROUTINE_OPERATORS
            .iter()
            .chain(INDONESIAN_ROUTINE_OPERATORS),
    );
    let mut formal_concepts = matched_words(&lower_active, FORMAL_CONCEPTS.iter());
    let mut suppressed_operators = matched_words(
        &lower_negated,
        ENGLISH_DEMANDING_OPERATORS
            .iter()
            .chain(INDONESIAN_DEMANDING_OPERATORS)
            .chain(ENGLISH_ROUTINE_OPERATORS)
            .chain(INDONESIAN_ROUTINE_OPERATORS),
    );
    demanding_operators.sort();
    demanding_operators.dedup();
    routine_operators.sort();
    routine_operators.dedup();
    formal_concepts.sort();
    formal_concepts.dedup();
    suppressed_operators.sort();
    suppressed_operators.dedup();

    let mut spans = active
        .into_iter()
        .map(|range| span(range, InstructionSpanKind::Active))
        .chain(
            quoted_data
                .into_iter()
                .map(|range| span(range, InstructionSpanKind::QuotedData)),
        )
        .chain(
            negated
                .into_iter()
                .map(|range| span(range, InstructionSpanKind::Negated)),
        )
        .collect::<Vec<_>>();
    spans.sort_by_key(|span| (span.start, span.end, span.kind));

    ActiveInstructionAnalysis {
        active_text,
        lexical_text,
        evidence: ActiveInstructionEvidence {
            analysis_version: MERIDIAN_ACTIVE_INSTRUCTION_VERSION.to_string(),
            language_profile: language_profile(&lower_active),
            normalized_input_sha256: format!("{:x}", Sha256::digest(normalized.as_bytes())),
            normalized_input_bytes: u32::try_from(normalized.len()).unwrap_or(u32::MAX),
            active_instruction_tokens,
            quoted_data_tokens: estimate_tokens(&quoted_text),
            spans,
            demanding_operators,
            routine_operators,
            formal_concepts,
            suppressed_operators,
        },
    }
}

fn normalize_messages(messages: &[Message]) -> (String, Vec<MessageRegion>) {
    let mut normalized = String::new();
    let mut regions = Vec::new();
    for message in messages {
        let text = extract_content_text(&message.content);
        if text.trim().is_empty() {
            continue;
        }
        if !normalized.is_empty() {
            normalized.push('\n');
        }
        normalized.push_str(role_label(&message.role));
        normalized.push('\n');
        let start = normalized.len();
        normalized.push_str(&text);
        regions.push(MessageRegion {
            range: start..normalized.len(),
            active_role: matches!(message.role, Role::System | Role::User),
        });
    }
    (normalized, regions)
}

fn role_label(role: &Role) -> &'static str {
    match role {
        Role::System => "[system]",
        Role::User => "[user]",
        Role::Assistant => "[assistant]",
        Role::Function => "[function]",
        Role::Tool => "[tool]",
    }
}

fn extract_content_text(content: &Content) -> String {
    match content {
        Content::Text(text) => text.clone(),
        Content::Parts(parts) => parts
            .iter()
            .filter_map(|part| match part {
                ContentPart::Text { text } => Some(text.as_str()),
                ContentPart::ImageUrl { .. } => None,
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn quoted_data_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = declared_data_ranges(text);
    ranges.extend(paired_delimited_ranges(text, '"', '"', false));
    ranges.extend(paired_delimited_ranges(text, '“', '”', false));
    ranges.extend(paired_delimited_ranges(text, '\'', '\'', true));
    ranges.extend(paired_delimited_ranges(text, '‘', '’', true));
    ranges.extend(paired_delimited_ranges(text, '`', '`', false));
    merge_ranges(ranges)
}

fn declared_data_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = fenced_ranges(text);
    ranges.extend(block_quote_ranges(text));
    if let Some(range) = marked_data_payload(text) {
        ranges.push(range);
    }
    merge_ranges(ranges)
}

fn fenced_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut cursor = 0;
    while let Some(relative_start) = text[cursor..].find("```") {
        let start = cursor + relative_start;
        let after_start = start + 3;
        let end = text[after_start..]
            .find("```")
            .map(|relative_end| after_start + relative_end + 3)
            .unwrap_or(text.len());
        ranges.push(start..end);
        cursor = end;
    }
    ranges
}

fn paired_delimited_ranges(
    text: &str,
    opening: char,
    closing: char,
    apostrophe_aware: bool,
) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = None;
    for (index, character) in text.char_indices() {
        let expected = if start.is_some() { closing } else { opening };
        if character != expected
            || is_escaped(text, index)
            || (apostrophe_aware && is_intra_word_apostrophe(text, index))
        {
            continue;
        }
        if let Some(opening_index) = start.take() {
            ranges.push(opening_index..index + character.len_utf8());
        } else {
            start = Some(index);
        }
    }
    ranges
}

fn is_escaped(text: &str, index: usize) -> bool {
    text[..index]
        .bytes()
        .rev()
        .take_while(|byte| *byte == b'\\')
        .count()
        % 2
        == 1
}

fn is_intra_word_apostrophe(text: &str, index: usize) -> bool {
    text[..index]
        .chars()
        .next_back()
        .is_some_and(char::is_alphanumeric)
        && text[index..]
            .chars()
            .nth(1)
            .is_some_and(char::is_alphanumeric)
}

fn block_quote_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with('>') {
            ranges.push(offset..offset + line.len());
        }
        offset += line.len();
    }
    ranges
}

fn marked_data_payload(text: &str) -> Option<Range<usize>> {
    let colon = text.find(':')?;
    let prefix = text[..colon].to_ascii_lowercase();
    if !DATA_MARKERS.iter().any(|marker| prefix.contains(marker)) {
        return None;
    }
    let payload_start = colon + 1;
    let lower_payload = text[payload_start..].to_ascii_lowercase();
    let payload_end = DATA_FOLLOW_ON_BOUNDARIES
        .iter()
        .filter_map(|marker| lower_payload.find(marker))
        .min()
        .map(|boundary| payload_start + boundary + 1)
        .unwrap_or(text.len());
    Some(payload_start..payload_end)
}

fn negated_ranges(text: &str) -> Vec<Range<usize>> {
    // Every supported negation and contrast marker is ASCII. ASCII-only case
    // folding preserves byte offsets into the original UTF-8 request.
    let lower = text.to_ascii_lowercase();
    let mut ranges = Vec::new();
    let mut cursor = 0;
    while cursor < lower.len() {
        let next = NEGATION_PREFIXES
            .iter()
            .filter_map(|prefix| {
                lower[cursor..]
                    .find(prefix)
                    .map(|relative| (cursor + relative, *prefix))
            })
            .min_by_key(|(start, _)| *start);
        let Some((start, prefix)) = next else {
            break;
        };
        let search_start = start + prefix.len();
        if matches!(prefix, "without " | "tanpa ")
            && !starts_with_supported_operator(&lower[search_start..])
        {
            cursor = search_start;
            continue;
        }
        let scope_start = start;
        let punctuation = lower[search_start..]
            .char_indices()
            .find(|(_, character)| matches!(character, '.' | ';' | '!' | '?' | '\n' | '\r'))
            .map(|(relative, character)| search_start + relative + character.len_utf8());
        let contrast = CONTRAST_BOUNDARIES
            .iter()
            .filter_map(|marker| {
                lower[search_start..]
                    .find(marker)
                    .map(|relative| search_start + relative)
            })
            .min();
        let end = punctuation
            .into_iter()
            .chain(contrast)
            .min()
            .unwrap_or(lower.len());
        ranges.push(scope_start..end);
        cursor = end.max(search_start);
    }
    merge_ranges(ranges)
}

fn starts_with_supported_operator(scope: &str) -> bool {
    let first_word = scope
        .trim_start()
        .split(|character: char| !(character.is_alphanumeric() || character == '-'))
        .next()
        .unwrap_or_default();
    ENGLISH_DEMANDING_OPERATORS
        .iter()
        .chain(INDONESIAN_DEMANDING_OPERATORS)
        .chain(ENGLISH_ROUTINE_OPERATORS)
        .chain(INDONESIAN_ROUTINE_OPERATORS)
        .any(|operator| {
            first_word == *operator
                || operator
                    .strip_suffix('e')
                    .is_some_and(|stem| first_word == format!("{stem}ing"))
                || first_word == format!("{operator}ing")
        })
}

fn complement_ranges(
    eligible: &[Range<usize>],
    quoted_data: &[Range<usize>],
    negated: &[Range<usize>],
) -> Vec<Range<usize>> {
    let excluded = merge_ranges(
        quoted_data
            .iter()
            .cloned()
            .chain(negated.iter().cloned())
            .collect(),
    );
    let mut active = Vec::new();
    for region in eligible {
        let mut cursor = region.start;
        for exclusion in excluded
            .iter()
            .filter(|range| ranges_overlap(range, region))
        {
            let start = exclusion.start.max(region.start);
            let end = exclusion.end.min(region.end);
            if cursor < start {
                active.push(cursor..start);
            }
            cursor = cursor.max(end);
        }
        if cursor < region.end {
            active.push(cursor..region.end);
        }
    }
    active
        .into_iter()
        .filter(|range| range.start < range.end)
        .collect()
}

fn merge_ranges(mut ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    ranges.retain(|range| range.start < range.end);
    ranges.sort_by_key(|range| (range.start, range.end));
    let mut merged: Vec<Range<usize>> = Vec::new();
    for range in ranges {
        if let Some(previous) = merged.last_mut() {
            if range.start <= previous.end {
                previous.end = previous.end.max(range.end);
                continue;
            }
        }
        merged.push(range);
    }
    merged
}

fn joined_ranges(text: &str, ranges: &[Range<usize>]) -> String {
    ranges
        .iter()
        .map(|range| text[range.clone()].trim())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn matched_words<'a>(lower: &str, words: impl Iterator<Item = &'a &'static str>) -> Vec<String> {
    words
        .filter(|word| contains_phrase(lower, word))
        .map(|word| (*word).to_string())
        .collect()
}

fn contains_phrase(lower: &str, phrase: &str) -> bool {
    if phrase.contains(' ') {
        return lower
            .split(|character: char| !(character.is_alphanumeric() || character == '-'))
            .collect::<Vec<_>>()
            .windows(phrase.split_whitespace().count())
            .any(|window| window.join(" ") == phrase);
    }
    lower
        .split(|character: char| !(character.is_alphanumeric() || character == '-'))
        .any(|word| word == phrase)
}

fn language_profile(lower: &str) -> InstructionLanguageProfile {
    let english = matched_count(lower, ENGLISH_LANGUAGE_MARKERS)
        + matched_count(lower, ENGLISH_DEMANDING_OPERATORS)
        + matched_count(lower, ENGLISH_ROUTINE_OPERATORS);
    let indonesian = matched_count(lower, INDONESIAN_LANGUAGE_MARKERS)
        + matched_count(lower, INDONESIAN_DEMANDING_OPERATORS)
        + matched_count(lower, INDONESIAN_ROUTINE_OPERATORS);
    match (english > 0, indonesian > 0) {
        (true, true) => InstructionLanguageProfile::Mixed,
        (true, false) => InstructionLanguageProfile::English,
        (false, true) => InstructionLanguageProfile::Indonesian,
        (false, false) => InstructionLanguageProfile::Unsupported,
    }
}

fn matched_count(lower: &str, words: &[&str]) -> usize {
    words
        .iter()
        .filter(|word| contains_phrase(lower, word))
        .count()
}

fn estimate_tokens(text: &str) -> u32 {
    let words = text.split_whitespace().count();
    ((words as f32) / 0.75).round() as u32
}

fn span(range: Range<usize>, kind: InstructionSpanKind) -> InstructionSpanEvidence {
    InstructionSpanEvidence {
        start: u32::try_from(range.start).unwrap_or(u32::MAX),
        end: u32::try_from(range.end).unwrap_or(u32::MAX),
        kind,
    }
}

fn shift_range(range: Range<usize>, offset: usize) -> Range<usize> {
    range.start + offset..range.end + offset
}

fn ranges_overlap(left: &Range<usize>, right: &Range<usize>) -> bool {
    left.start < right.end && right.start < left.end
}

fn is_canonical_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
