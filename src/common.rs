use serde::Deserialize;
use std::collections::HashMap;

/// Original Amp structure from JSON
#[derive(Clone, Debug, Deserialize)]
pub struct OriginalAmp {
    pub keywords: Vec<String>,
    pub title: String,
    pub url: String,
    pub score: Option<f64>,
    #[serde(default)]
    pub full_keywords: Vec<(String, usize)>,
    pub advertiser: String,
    #[serde(rename = "id")]
    pub block_id: i32,
    pub iab_category: String,
    pub click_url: String,
    pub impression_url: String,
    #[serde(rename = "icon")]
    pub icon_id: String,
}

/// Common result structure
#[derive(Clone, Debug)]
pub struct AmpResult {
    pub title: String,
    pub url: String,
    pub click_url: String,
    pub impression_url: String,
    pub advertiser: String,
    pub block_id: i32,
    pub iab_category: String,
    pub icon: String,
    pub full_keyword: String,
}

/// Interface for all AMP indexers
pub trait AmpIndexer {
    /// Create a new index
    fn new() -> Self;

    /// Build the index from raw AMP data
    fn build(&mut self, amps: &[OriginalAmp]) -> Result<(), Box<dyn std::error::Error>>;

    /// Query for suggestions matching a prefix
    fn query(&self, prefix: &str) -> Result<Vec<AmpResult>, Box<dyn std::error::Error>>;

    /// Get statistics about the index
    fn stats(&self) -> HashMap<String, usize>;
}

/// Run-End encoding for full keywords
pub struct RunEndEncoding {
    pub values: Vec<String>,
    pub indices: Vec<usize>,
}

impl RunEndEncoding {
    pub fn new() -> Self {
        RunEndEncoding {
            values: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn add(&mut self, value: String, count: usize) {
        self.values.push(value);
        let next_index = self.indices.last().map_or(count - 1, |last| last + count);
        self.indices.push(next_index);
    }

    pub fn get(&self, index: usize) -> Option<&str> {
        // Binary search to find the right segment
        match self.indices.binary_search(&index) {
            Ok(exact_idx) => Some(&self.values[exact_idx]),
            Err(insertion_idx) => {
                if insertion_idx == 0 {
                    None // index is before the first element
                } else {
                    Some(&self.values[insertion_idx - 1])
                }
            }
        }
    }
}

/// Dictionary encoding for URLs
pub fn extract_template(
    url: &str,
    template_lookup: &mut HashMap<String, u32>,
    templates: &mut HashMap<u32, String>,
) -> (u32, String) {
    let split_idx = url.find('?').unwrap_or_else(|| url.rfind('/').unwrap_or(0));
    let (template, suffix) = url.split_at(split_idx);

    match template_lookup.get(template) {
        Some(&id) => (id, suffix.to_string()),
        None => {
            let id = template_lookup.len() as u32;
            template_lookup.insert(template.to_string(), id);
            templates.insert(id, template.to_string());
            (id, suffix.to_string())
        }
    }
}

/// Collapse each maximal chain of one-char extensions into its last element,
/// while preserving how many characters the user must type (min_prefix_len)
/// to hit that collapsed key.
/// e.g. ["fo","foo","foob","fooba","foobar"] → [("foobar", 2)]
pub fn collapse_keywords(keywords: &[String]) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < keywords.len() {
        let curr = &keywords[i];

        let curr_len = curr.chars().count();
        let mut j = i + 1;

        // extend the run as long as each next is curr + exactly one char
        while j < keywords.len() {
            let nxt = &keywords[j];
            if nxt.starts_with(curr) && nxt.chars().count() == curr_len + 1 {
                j += 1;
            } else {
                break;
            }
        }
        if j > i + 1 {
            // we saw a run [i .. j), so collapse to keywords[j-1]
            out.push((keywords[j - 1].clone(), curr_len));
            i = j;
        } else {
            out.push((curr.clone(), curr_len));
            i += 1;
        }
    }
    out
}
