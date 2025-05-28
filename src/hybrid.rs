use crate::common::{
    AmpIndexer, AmpResult, OriginalAmp, RunEndEncoding, collapse_keywords, extract_template,
};
use qp_trie::Trie;
use std::collections::HashMap;

/// Compact AMP suggestion with maximum dictionary encoding
#[derive(Clone, Debug)]
struct CompactAmpSuggestion {
    title_id: u32, // Dictionary-encoded title
    url_template_id: u32,
    url_suffix: String,
    click_url_template_id: u32,
    click_url_suffix: String,
    impression_url_template_id: u32,
    impression_url_suffix: String,
    advertiser_id: u32,
    block_id: i32,
    iab_category_id: u32, // Dictionary-encoded IAB category
    icon_id: u32,         // Dictionary-encoded icon
}

/// Value stored in the hybrid index
#[derive(Clone, Debug)]
struct IndexValue {
    suggestion_idx: usize,
    full_kw_idx: usize,
    min_prefix_len: usize,
}

/// Fast lookup cache for very short prefixes
#[derive(Debug)]
struct ShortPrefixCache {
    /// Direct cache mapping short keywords to their values
    exact_matches: HashMap<String, IndexValue>,
}

impl ShortPrefixCache {
    fn new() -> Self {
        ShortPrefixCache {
            exact_matches: HashMap::new(),
        }
    }

    fn insert(&mut self, key: &str, value: IndexValue) {
        // Only store exact matches, no duplicates
        self.exact_matches.insert(key.to_string(), value);
    }

    fn lookup(&self, query: &str, query_len: usize) -> Option<&IndexValue> {
        // Try exact match first
        if let Some(value) = self.exact_matches.get(query) {
            if query_len >= value.min_prefix_len {
                return Some(value);
            }
        }

        // Try prefix matches - find the shortest key that starts with the query
        let mut best_match: Option<&IndexValue> = None;
        let mut best_key_len = usize::MAX;

        for (key, value) in &self.exact_matches {
            if key.starts_with(query) && query_len >= value.min_prefix_len {
                if key.len() < best_key_len {
                    best_match = Some(value);
                    best_key_len = key.len();
                }
            }
        }

        best_match
    }
}

/// Hybrid AMP Index combining multiple optimization strategies
pub struct HybridAmpIndex {
    /// QP-trie for efficient prefix matching of longer keys
    main_trie: Trie<Vec<u8>, IndexValue>,

    /// Fast cache for very short prefixes (1-3 chars)
    short_cache: ShortPrefixCache,

    /// Compact suggestion storage with maximum dictionary encoding
    suggestions: Vec<CompactAmpSuggestion>,

    /// Run-end encoding for full keywords
    full_keywords: RunEndEncoding,

    /// Dictionary structures for all repeated fields
    advertisers: HashMap<u32, String>,
    titles: HashMap<u32, String>,
    url_templates: HashMap<u32, String>,
    click_url_templates: HashMap<u32, String>,
    impression_url_templates: HashMap<u32, String>,
    iab_categories: HashMap<u32, String>,
    icons: HashMap<u32, String>,

    /// Statistics
    keyword_count: usize,
}

impl AmpIndexer for HybridAmpIndex {
    fn new() -> Self {
        HybridAmpIndex {
            main_trie: Trie::new(),
            short_cache: ShortPrefixCache::new(),
            suggestions: Vec::new(),
            full_keywords: RunEndEncoding::new(),
            advertisers: HashMap::new(),
            titles: HashMap::new(),
            url_templates: HashMap::new(),
            click_url_templates: HashMap::new(),
            impression_url_templates: HashMap::new(),
            iab_categories: HashMap::new(),
            icons: HashMap::new(),
            keyword_count: 0,
        }
    }

    fn build(&mut self, amps: &[OriginalAmp]) -> Result<(), Box<dyn std::error::Error>> {
        // Dictionary lookup tables for building phase
        let mut advertiser_lookup = HashMap::new();
        let mut title_lookup = HashMap::new();
        let mut url_lookup = HashMap::new();
        let mut click_lookup = HashMap::new();
        let mut imp_lookup = HashMap::new();
        let mut iab_lookup = HashMap::new();
        let mut icon_lookup = HashMap::new();

        for amp in amps {
            // Dictionary encode all repeated fields - using static method to avoid borrowing conflicts
            let advertiser_id = Self::intern_string_static(
                &amp.advertiser,
                &mut advertiser_lookup,
                &mut self.advertisers,
            );
            let title_id =
                Self::intern_string_static(&amp.title, &mut title_lookup, &mut self.titles);
            let iab_id = Self::intern_string_static(
                &amp.iab_category,
                &mut iab_lookup,
                &mut self.iab_categories,
            );
            let icon_id =
                Self::intern_string_static(&amp.icon_id, &mut icon_lookup, &mut self.icons);

            // URL template extraction
            let (url_tid, url_suf) =
                extract_template(&amp.url, &mut url_lookup, &mut self.url_templates);
            let (clk_tid, clk_suf) = extract_template(
                &amp.click_url,
                &mut click_lookup,
                &mut self.click_url_templates,
            );
            let (imp_tid, imp_suf) = extract_template(
                &amp.impression_url,
                &mut imp_lookup,
                &mut self.impression_url_templates,
            );

            // Store compact suggestion
            let sidx = self.suggestions.len();
            self.suggestions.push(CompactAmpSuggestion {
                title_id,
                url_template_id: url_tid,
                url_suffix: url_suf,
                click_url_template_id: clk_tid,
                click_url_suffix: clk_suf,
                impression_url_template_id: imp_tid,
                impression_url_suffix: imp_suf,
                advertiser_id,
                block_id: amp.block_id,
                iab_category_id: iab_id,
                icon_id,
            });

            // Encode full keywords
            let fkw_start = self.full_keywords.indices.len();
            if !amp.full_keywords.is_empty() {
                for (full_kw, count) in &amp.full_keywords {
                    self.full_keywords.add(full_kw.clone(), *count);
                }
            } else {
                self.full_keywords.add(amp.advertiser.clone(), 1);
            }

            // Process collapsed keywords and distribute between cache and trie
            if !amp.keywords.is_empty() {
                for (i, (kw, min_pref)) in collapse_keywords(&amp.keywords).into_iter().enumerate()
                {
                    let value = IndexValue {
                        suggestion_idx: sidx,
                        full_kw_idx: fkw_start + i,
                        min_prefix_len: min_pref,
                    };

                    let kw_chars: Vec<char> = kw.chars().collect();

                    // Short keys (including those with spaces) go to cache, longer keys go to trie
                    if kw_chars.len() <= 3 {
                        self.short_cache.insert(&kw, value);
                    } else {
                        // Convert to bytes for QP-trie
                        self.main_trie.insert(kw.as_bytes().to_vec(), value);
                    }

                    self.keyword_count += 1;
                }
            }
        }

        // Optimize cache by sorting entries by relevance
        self.optimize_cache();

        self.suggestions.shrink_to_fit();

        Ok(())
    }

    fn query(&self, query: &str) -> Result<Vec<AmpResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        // Don't trim the query - preserve spaces as they might be significant
        let qlen = query.chars().count();

        // First try the short prefix cache for very fast lookups
        if qlen <= 3 {
            if let Some(value) = self.short_cache.lookup(query, qlen) {
                if qlen >= value.min_prefix_len {
                    self.build_result(value.suggestion_idx, value.full_kw_idx, &mut results)?;
                    return Ok(results);
                }
            }
        }

        // Fall back to trie for longer queries or cache misses
        let query_bytes = query.as_bytes();

        // Try exact match first
        if let Some(value) = self.main_trie.get(query_bytes) {
            if qlen >= value.min_prefix_len {
                self.build_result(value.suggestion_idx, value.full_kw_idx, &mut results)?;
                return Ok(results);
            }
        }

        // Prefix search with optimization for shortest match
        let mut best_match: Option<&IndexValue> = None;
        let mut best_len = usize::MAX;

        for (key, value) in self.main_trie.iter_prefix(query_bytes) {
            if qlen >= value.min_prefix_len {
                if key.len() < best_len {
                    best_match = Some(value);
                    best_len = key.len();
                }
            }
        }

        if let Some(value) = best_match {
            self.build_result(value.suggestion_idx, value.full_kw_idx, &mut results)?;
        }

        Ok(results)
    }

    fn stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        stats.insert("keyword_count".into(), self.keyword_count);
        stats.insert("suggestions_count".into(), self.suggestions.len());
        stats.insert(
            "full_keywords_count".into(),
            self.full_keywords.indices.len(),
        );
        stats.insert("advertisers_count".into(), self.advertisers.len());
        stats.insert("titles_count".into(), self.titles.len());
        stats.insert("url_templates_count".into(), self.url_templates.len());
        stats.insert("iab_categories_count".into(), self.iab_categories.len());
        stats.insert("icons_count".into(), self.icons.len());
        stats.insert(
            "cache_exact_matches".into(),
            self.short_cache.exact_matches.len(),
        );
        // Note: qp-trie doesn't have a len() method, so we estimate from keyword_count
        stats.insert(
            "main_trie_estimated_size".into(),
            self.keyword_count
                .saturating_sub(self.short_cache.exact_matches.len()),
        );
        stats
    }
}

impl HybridAmpIndex {
    /// Intern a string into a dictionary, returning its ID (static version to avoid borrowing conflicts)
    fn intern_string_static(
        value: &str,
        lookup: &mut HashMap<String, u32>,
        dict: &mut HashMap<u32, String>,
    ) -> u32 {
        if let Some(&id) = lookup.get(value) {
            id
        } else {
            let id = lookup.len() as u32;
            lookup.insert(value.to_string(), id);
            dict.insert(id, value.to_string());
            id
        }
    }

    /// Optimize the cache by sorting and deduplicating entries
    fn optimize_cache(&mut self) {
        // The new cache structure doesn't need optimization since it uses exact matches only
        // No duplicates possible with HashMap
    }

    /// Build a result from the compact storage
    fn build_result(
        &self,
        sugg_idx: usize,
        fkw_idx: usize,
        results: &mut Vec<AmpResult>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(sug) = self.suggestions.get(sugg_idx) {
            // Reconstruct all fields from dictionaries
            let title = self.titles.get(&sug.title_id).cloned().unwrap_or_default();
            let advertiser = self
                .advertisers
                .get(&sug.advertiser_id)
                .cloned()
                .unwrap_or_default();
            let iab_category = self
                .iab_categories
                .get(&sug.iab_category_id)
                .cloned()
                .unwrap_or_default();
            let icon = self.icons.get(&sug.icon_id).cloned().unwrap_or_default();

            let full_keyword = match self.full_keywords.get(fkw_idx) {
                Some(kw) => kw.to_string(),
                None => advertiser.clone(),
            };

            // Reconstruct URLs
            let url =
                self.reconstruct_url(sug.url_template_id, &sug.url_suffix, &self.url_templates);
            let click_url = self.reconstruct_url(
                sug.click_url_template_id,
                &sug.click_url_suffix,
                &self.click_url_templates,
            );
            let impression_url = self.reconstruct_url(
                sug.impression_url_template_id,
                &sug.impression_url_suffix,
                &self.impression_url_templates,
            );

            results.push(AmpResult {
                title,
                url,
                click_url,
                impression_url,
                advertiser,
                block_id: sug.block_id,
                iab_category,
                icon,
                full_keyword,
            });
        }
        Ok(())
    }

    fn reconstruct_url(
        &self,
        template_id: u32,
        suffix: &str,
        dict: &HashMap<u32, String>,
    ) -> String {
        dict.get(&template_id)
            .map(|t| format!("{}{}", t, suffix))
            .unwrap_or_else(|| suffix.to_string())
    }
}
