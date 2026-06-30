use crate::normalize::regex::RegexNormalizer;

pub fn timestamp_normalizer() -> RegexNormalizer {
    RegexNormalizer::new(
        "timestamp",
        r"\b\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z\b",
        "<TIMESTAMP>",
    )
    .expect("built-in timestamp regex is valid")
}
