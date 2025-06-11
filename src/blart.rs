use crate::common::{
    AmpIndexer, AmpResult, FullKeyword, OriginalAmp, collapse_keywords_ex, extract_template,
};
use blart::TreeMap;
use std::collections::HashMap;

/// Stores the metadata for each collapsed keyword
#[derive(Clone)]
struct KeywordMetadata {
    suggestion_idx: usize,
    min_prefix_len: usize,
    full_keyword: FullKeyword,
    collapsed_keyword: String,
}

/// Same compact suggestion structure as our other implementations
#[derive(Clone)]
struct CompactSuggestion {
    title_id: u32,
    url_tid: u32,
    url_suffix: String,
    click_tid: u32,
    click_suffix: String,
    imp_tid: u32,
    imp_suffix: String,
    advertiser_id: u32,
    block_id: i32,
    iab_id: u32,
    icon_id: u32,
}

/// AMP Index using BLART (Adaptive Radix Tree)
pub struct BlartAmpIndex {
    /// BLART handles all the complex tree operations for us
    keyword_tree: TreeMap<Box<[u8]>, KeywordMetadata>,

    /// Storage for suggestions
    suggestions: Vec<CompactSuggestion>,

    /// Dictionary structures - identical to other implementations
    advertisers: HashMap<u32, String>,
    titles: HashMap<u32, String>,
    url_templates: HashMap<u32, String>,
    click_templates: HashMap<u32, String>,
    imp_templates: HashMap<u32, String>,
    iab_categories: HashMap<u32, String>,
    icons: HashMap<u32, String>,
}

impl AmpIndexer for BlartAmpIndex {
    fn new() -> Self {
        BlartAmpIndex {
            keyword_tree: TreeMap::new(),
            suggestions: Vec::new(),
            advertisers: HashMap::new(),
            titles: HashMap::new(),
            url_templates: HashMap::new(),
            click_templates: HashMap::new(),
            imp_templates: HashMap::new(),
            iab_categories: HashMap::new(),
            icons: HashMap::new(),
        }
    }

    fn build(&mut self, amps: &[OriginalAmp]) -> Result<(), Box<dyn std::error::Error>> {
        // Dictionary lookups - same pattern as other implementations
        let mut adv_lookup = HashMap::new();
        let mut title_lookup = HashMap::new();
        let mut url_lookup = HashMap::new();
        let mut click_lookup = HashMap::new();
        let mut imp_lookup = HashMap::new();
        let mut iab_lookup = HashMap::new();
        let mut icon_lookup = HashMap::new();

        for amp in amps {
            // Dictionary encode all fields
            let advertiser_id =
                Self::intern(&amp.advertiser, &mut adv_lookup, &mut self.advertisers);
            let title_id = Self::intern(&amp.title, &mut title_lookup, &mut self.titles);
            let iab_id = Self::intern(&amp.iab_category, &mut iab_lookup, &mut self.iab_categories);
            let icon_id = Self::intern(&amp.icon_id, &mut icon_lookup, &mut self.icons);

            // Extract URL templates
            let (url_tid, url_suf) =
                extract_template(&amp.url, &mut url_lookup, &mut self.url_templates);
            let (click_tid, clk_suf) =
                extract_template(&amp.click_url, &mut click_lookup, &mut self.click_templates);
            let (imp_tid, imp_suf) = extract_template(
                &amp.impression_url,
                &mut imp_lookup,
                &mut self.imp_templates,
            );

            // Store suggestion
            let sidx = self.suggestions.len();
            self.suggestions.push(CompactSuggestion {
                title_id,
                url_tid,
                url_suffix: url_suf,
                click_tid,
                click_suffix: clk_suf,
                imp_tid,
                imp_suffix: imp_suf,
                advertiser_id,
                block_id: amp.block_id,
                iab_id,
                icon_id,
            });

            // Process and insert collapsed keywords
            for (kw, min_pref, full_kw) in collapse_keywords_ex(&amp.keywords, &amp.full_keywords) {
                let metadata = KeywordMetadata {
                    suggestion_idx: sidx,
                    min_prefix_len: min_pref,
                    full_keyword: full_kw,
                    collapsed_keyword: kw.clone(),
                };

                // BLART requires Box<[u8]> for keys
                let key = kw.into_bytes().into_boxed_slice();

                // BLART returns the old value if key already exists
                // We'll keep the first occurrence (like other implementations)
                let _ = self.keyword_tree.try_insert(key, metadata);
            }
        }

        Ok(())
    }

    fn query(&self, query: &str) -> Result<Vec<AmpResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();
        let query_len = query.chars().count();
        let query_bytes = query.as_bytes();

        // Find the shortest valid match using BLART's range iterator
        let mut best_match: Option<(&[u8], &KeywordMetadata)> = None;

        // Convert query to Box<[u8]> for the range bound
        let range_start = query_bytes.to_vec().into_boxed_slice();

        // Use the boxed slice as the range start
        for (key, metadata) in self.keyword_tree.range(range_start..) {
            // Check if this key actually starts with our query
            if !key.starts_with(query_bytes) {
                break; // No more matches possible
            }

            // Check minimum prefix length requirement
            if query_len >= metadata.min_prefix_len {
                // Take the first valid match (shortest due to tree ordering)
                best_match = Some((key, metadata));
                break;
            }
        }

        // Build result if we found a match
        if let Some((_, metadata)) = best_match {
            self.build_result(metadata, &mut results)?;
        }

        Ok(results)
    }

    fn stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();

        stats.insert("keyword_count".into(), self.keyword_tree.len());
        stats.insert("suggestions_count".into(), self.suggestions.len());
        stats.insert("advertisers_count".into(), self.advertisers.len());
        stats.insert("titles_count".into(), self.titles.len());
        stats.insert("url_templates_count".into(), self.url_templates.len());
        stats.insert("iab_categories_count".into(), self.iab_categories.len());
        stats.insert("icons_count".into(), self.icons.len());

        stats
    }
}

impl BlartAmpIndex {
    /// Helper for string interning
    fn intern(
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

    /// Build result from metadata and dictionaries
    fn build_result(
        &self,
        metadata: &KeywordMetadata,
        results: &mut Vec<AmpResult>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sug = &self.suggestions[metadata.suggestion_idx];

        // Reconstruct all fields from dictionaries
        let title = self.titles.get(&sug.title_id).cloned().unwrap_or_default();
        let advertiser = self
            .advertisers
            .get(&sug.advertiser_id)
            .cloned()
            .unwrap_or_default();
        let iab_category = self
            .iab_categories
            .get(&sug.iab_id)
            .cloned()
            .unwrap_or_default();
        let icon = self.icons.get(&sug.icon_id).cloned().unwrap_or_default();

        // Handle full keyword
        let full_keyword = match &metadata.full_keyword {
            FullKeyword::Same => {
                // Use the stored collapsed keyword directly
                metadata.collapsed_keyword.clone()
            }
            FullKeyword::Different(kw) => kw.clone(),
        };

        // Reconstruct URLs
        let url = self.reconstruct_url(sug.url_tid, &sug.url_suffix, &self.url_templates);
        let click_url =
            self.reconstruct_url(sug.click_tid, &sug.click_suffix, &self.click_templates);
        let impression_url =
            self.reconstruct_url(sug.imp_tid, &sug.imp_suffix, &self.imp_templates);

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

        Ok(())
    }

    fn reconstruct_url(
        &self,
        template_id: u32,
        suffix: &str,
        templates: &HashMap<u32, String>,
    ) -> String {
        templates
            .get(&template_id)
            .map(|t| format!("{}{}", t, suffix))
            .unwrap_or_else(|| suffix.to_string())
    }
}
