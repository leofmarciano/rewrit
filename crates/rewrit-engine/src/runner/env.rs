use regex::Regex;

#[derive(Debug, Clone)]
pub struct Redactor {
    patterns: Vec<Regex>,
}

impl Redactor {
    #[must_use]
    pub fn new(patterns: &[String]) -> Self {
        let patterns = patterns
            .iter()
            .filter_map(|pattern| Regex::new(pattern).ok())
            .collect();
        Self { patterns }
    }

    #[must_use]
    pub fn redact(&self, input: &str) -> String {
        self.patterns
            .iter()
            .fold(input.to_string(), |acc, pattern| {
                pattern.replace_all(&acc, "<REDACTED>").into_owned()
            })
    }
}

pub fn truncate(mut text: String, max_bytes: usize) -> (String, bool) {
    if text.len() <= max_bytes {
        return (text, false);
    }

    while !text.is_char_boundary(max_bytes) {
        text.truncate(max_bytes - 1);
    }
    text.truncate(max_bytes);
    (text, true)
}
