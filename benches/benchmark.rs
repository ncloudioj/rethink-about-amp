use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use rethink_about_amp::*;
use std::time::Duration;

// Create benchmark data once
fn create_benchmark_data() -> Vec<OriginalAmp> {
    // Try to load real data, fall back to synthetic data
    load_amp_data("data/amp-us-desktop.json").unwrap_or_else(|_| {
        // Create synthetic data for benchmarking if real data not available
        let mut synthetic_data = Vec::new();
        let advertisers = ["Amazon", "Wayfair", "Target", "eBay", "Walmart"];
        let categories = ["22 - Shopping", "18 - Technology", "19 - Travel"];

        for i in 0..1000 {
            let advertiser = advertisers[i % advertisers.len()];
            let category = categories[i % categories.len()];

            synthetic_data.push(OriginalAmp {
                keywords: vec![
                    format!("kw{}", i),
                    format!("kw{}_longer", i),
                    format!("keyword_{}", i),
                    format!("keyword_{}_extended", i),
                ],
                title: format!("{} - Product {}", advertiser, i),
                url: format!(
                    "https://www.{}.com/product/{}?tag=test",
                    advertiser.to_lowercase(),
                    i
                ),
                score: Some(0.1 + (i as f64 % 10.0) / 10.0),
                full_keywords: vec![
                    (format!("keyword_{}", i), 5),
                    (format!("keyword_{}_extended", i), 3),
                ],
                advertiser: advertiser.to_string(),
                block_id: i as i32,
                iab_category: category.to_string(),
                click_url: format!(
                    "https://click.{}.com/{}?ref=test",
                    advertiser.to_lowercase(),
                    i
                ),
                impression_url: format!(
                    "https://imp.{}.com/{}?ref=test",
                    advertiser.to_lowercase(),
                    i
                ),
                icon_id: format!("icon_{}", i % 50),
            });
        }
        synthetic_data
    })
}

// Build index benchmark
fn build_benchmark(c: &mut Criterion) {
    let amp_data = create_benchmark_data();

    let mut group = c.benchmark_group("build");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(10);

    group.bench_function("hybrid", |b| {
        b.iter(|| {
            let mut index = HybridAmpIndex::new();
            index.build(black_box(&amp_data)).unwrap();
            black_box(index)
        })
    });

    group.bench_function("btree", |b| {
        b.iter(|| {
            let mut index = BTreeAmpIndex::new();
            index.build(black_box(&amp_data)).unwrap();
            black_box(index)
        })
    });

    group.bench_function("art", |b| {
        b.iter(|| {
            let mut index = BlartAmpIndex::new();
            index.build(black_box(&amp_data)).unwrap();
            black_box(index)
        })
    });

    group.finish();
}

// Query benchmark with different query lengths
fn query_benchmark(c: &mut Criterion) {
    let amp_data = create_benchmark_data();

    // Pre-build all indexes
    let mut hybrid_index = HybridAmpIndex::new();
    hybrid_index.build(&amp_data).unwrap();

    let mut btree_index = BTreeAmpIndex::new();
    btree_index.build(&amp_data).unwrap();

    let mut art_index = BlartAmpIndex::new();
    art_index.build(&amp_data).unwrap();

    // Test different query patterns
    let test_queries = vec![
        ("single_char", "a"),
        ("double_char", "am"),
        ("short_word", "ama"),
        ("medium_word", "amaz"),
        ("full_word", "amazon"),
        ("long_phrase", "amazon fresh"),
        ("synthetic_short", "kw"),
        ("synthetic_medium", "keyword"),
        ("synthetic_long", "keyword_123"),
    ];

    for (query_type, query) in test_queries {
        let mut group = c.benchmark_group(&format!("query/{}", query_type));
        group.measurement_time(Duration::from_secs(10));

        group.bench_with_input(BenchmarkId::new("hybrid", query), query, |b, q| {
            b.iter(|| black_box(hybrid_index.query(black_box(q)).unwrap()))
        });

        group.bench_with_input(BenchmarkId::new("btree", query), query, |b, q| {
            b.iter(|| black_box(btree_index.query(black_box(q)).unwrap()))
        });

        group.bench_with_input(BenchmarkId::new("art", query), query, |b, q| {
            b.iter(|| black_box(art_index.query(black_box(q)).unwrap()))
        });

        group.finish();
    }
}

// Memory usage estimation benchmark
fn memory_analysis_benchmark(c: &mut Criterion) {
    let amp_data = create_benchmark_data();

    c.bench_function("memory_analysis", |b| {
        b.iter(|| {
            // Build each index and measure approximate memory usage
            let mut hybrid_index = HybridAmpIndex::new();
            hybrid_index.build(black_box(&amp_data)).unwrap();
            let hybrid_stats = hybrid_index.stats();

            let mut btree_index = BTreeAmpIndex::new();
            btree_index.build(black_box(&amp_data)).unwrap();
            let btree_stats = btree_index.stats();

            let mut art_index = BlartAmpIndex::new();
            art_index.build(black_box(&amp_data)).unwrap();
            let art_stats = art_index.stats();

            black_box((hybrid_stats, btree_stats, art_stats))
        })
    });
}

// Prefix iteration benchmark (tests how well each structure handles prefix queries)
fn prefix_iteration_benchmark(c: &mut Criterion) {
    let amp_data = create_benchmark_data();

    let mut hybrid_index = HybridAmpIndex::new();
    hybrid_index.build(&amp_data).unwrap();

    let mut btree_index = BTreeAmpIndex::new();
    btree_index.build(&amp_data).unwrap();

    let mut art_index = BlartAmpIndex::new();
    art_index.build(&amp_data).unwrap();

    let prefix_queries = vec!["a", "am", "k", "kw", "keyword"];

    for prefix in prefix_queries {
        let mut group = c.benchmark_group(&format!("prefix_iter/{}", prefix));

        group.bench_with_input(BenchmarkId::new("hybrid", prefix), prefix, |b, p| {
            b.iter(|| {
                let results = hybrid_index.query(black_box(p)).unwrap();
                black_box(results.len())
            })
        });

        group.bench_with_input(BenchmarkId::new("btree", prefix), prefix, |b, p| {
            b.iter(|| {
                let results = btree_index.query(black_box(p)).unwrap();
                black_box(results.len())
            })
        });

        group.bench_with_input(BenchmarkId::new("art", prefix), prefix, |b, p| {
            b.iter(|| {
                let results = art_index.query(black_box(p)).unwrap();
                black_box(results.len())
            })
        });

        group.finish();
    }
}

criterion_group!(
    benches,
    build_benchmark,
    query_benchmark,
    memory_analysis_benchmark,
    prefix_iteration_benchmark
);
criterion_main!(benches);
