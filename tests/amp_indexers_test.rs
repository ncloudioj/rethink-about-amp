use rethink_about_amp::{AmpIndexer, BlartAmpIndex, BTreeAmpIndex, HybridAmpIndex, load_amp_data};
use std::path::Path;

fn prepare_btree_index() -> BTreeAmpIndex {
    let data_path = Path::new("data/amp-us-desktop.json");
    let amps = load_amp_data(data_path).expect("Failed to load AMP data");

    let mut index = BTreeAmpIndex::new();
    index.build(&amps).expect("Failed to build BTree index");
    index
}

fn prepare_hybrid_index() -> HybridAmpIndex {
    let data_path = Path::new("data/amp-us-desktop.json");
    let amps = load_amp_data(data_path).expect("Failed to load AMP data");

    let mut index = HybridAmpIndex::new();
    index.build(&amps).expect("Failed to build FST index");
    index
}

fn prepare_blart_index() -> BlartAmpIndex {
    let data_path = Path::new("data/amp-us-desktop.json");
    let amps = load_amp_data(data_path).expect("Failed to load AMP data");

    let mut index = BlartAmpIndex::new();
    index.build(&amps).expect("Failed to build FST index");
    index
}

fn test_amazon_prefix_queries_for<T: AmpIndexer>(index: &T, indexer_name: &str) {
    let test_cases = [
        ("am", 1),     // Should match Amazon
        ("ama", 1),    // Should match Amazon
        ("amaz", 1),   // Should match Amazon
        ("amazo", 1),  // Should match Amazon
        ("amazon", 1), // Should match Amazon
        ("k c", 0),    // Should not match anything
    ];

    for (query, expected_count) in test_cases {
        let results = index.query(query).expect("Query failed");
        assert_eq!(
            results.len(),
            expected_count,
            "{}: Query '{}' returned {} results, expected {}",
            indexer_name,
            query,
            results.len(),
            expected_count
        );
    }
}

fn test_query_urls_for<T: AmpIndexer>(index: &T, indexer_name: &str) {
    // Each tuple is (search query, expected URL substring or None if no match)
    let test_cases: &[(&str, Option<&str>)] = &[
        ("am", Some("amazon.com")),
        ("ama", Some("amazon.com")),
        ("amaz", Some("amazon.com")),
        ("amazo", Some("amazon.com")),
        ("amazon", Some("amazon.com")),
        ("fo", None),
        ("k c", None),
        ("k cup", Some("www.wayfair.com")),
        ("mini ", Some("www.homedepot.com")),
        ("mini s", None),
    ];

    for &(query, expected) in test_cases {
        let results = index.query(query).expect("Query failed");
        match expected {
            Some(substr) => {
                assert!(
                    !results.is_empty(),
                    "{}: Expected a result for query '{}', got none",
                    indexer_name,
                    query
                );
                let url = &results[0].url;
                assert!(
                    url.contains(substr),
                    "{}: For query '{}', url '{}' does not contain expected '{}'",
                    indexer_name,
                    query,
                    url,
                    substr
                );
            }
            None => {
                assert!(
                    results.is_empty(),
                    "{}: Expected no result for query '{}', but got {:#?}",
                    indexer_name,
                    query,
                    results
                );
            }
        }
    }
}

fn test_stats_for<T: AmpIndexer>(index: &T, indexer_name: &str) {
    let stats = index.stats();

    // Make sure we have suggestions
    let suggestions_count_key = if stats.contains_key("suggestions_count") {
        "suggestions_count"
    } else {
        // Fallback for BTree which might use a different key
        "keyword_index_size"
    };

    let count = stats.get(suggestions_count_key).unwrap_or(&0);
    assert!(
        *count > 0,
        "{}: Expected {} > 0, got {}",
        indexer_name,
        suggestions_count_key,
        count
    );

    println!("{} stats: {:?}", indexer_name, stats);
}

#[test]
fn test_btree_amazon_prefix_queries() {
    let index = prepare_btree_index();
    test_amazon_prefix_queries_for(&index, "BTree");
}

#[test]
fn test_btree_query_urls() {
    let index = prepare_btree_index();
    test_query_urls_for(&index, "BTree");
}

#[test]
fn test_btree_stats() {
    let index = prepare_btree_index();
    test_stats_for(&index, "BTree");
}

#[test]
fn test_hybrid_amazon_prefix_queries() {
    let index = prepare_hybrid_index();
    test_amazon_prefix_queries_for(&index, "Hybrid");
}

#[test]
fn test_hybrid_query_urls() {
    let index = prepare_hybrid_index();
    test_query_urls_for(&index, "Hybrid");
}

#[test]
fn test_hybrid_stats() {
    let index = prepare_hybrid_index();
    test_stats_for(&index, "Hybrid");
}

#[test]
fn test_blart_amazon_prefix_queries() {
    let index = prepare_blart_index();
    test_amazon_prefix_queries_for(&index, "Blart");
}

#[test]
fn test_blart_query_urls() {
    let index = prepare_blart_index();
    test_query_urls_for(&index, "Blart");
}

#[test]
fn test_blart_stats() {
    let index = prepare_blart_index();
    test_stats_for(&index, "Blart");
}
