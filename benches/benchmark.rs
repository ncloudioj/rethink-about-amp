use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rethink_about_amp::{
    AmpIndexer, BTreeAmpIndex, FstAmpIndex, load_amp_data,
};
use std::time::Duration;

fn bench_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("build");
    group.measurement_time(Duration::from_secs(10));

    // Load the AMP data
    let amps = load_amp_data("data/amp-us-desktop.json").unwrap();

    // Benchmark building each index
    group.bench_function("btree", |b| {
        b.iter(|| {
            let mut index = BTreeAmpIndex::new();
            index.build(&amps).unwrap();
        })
    });

    group.bench_function("fst", |b| {
        b.iter(|| {
            let mut index = FstAmpIndex::new();
            index.build(&amps).unwrap();
        })
    });

    group.finish();
}

fn bench_query(c: &mut Criterion) {
    // Load the AMP data
    let amps = load_amp_data("data/amp-us-desktop.json").unwrap();

    // Build each index once
    let mut btree_index = BTreeAmpIndex::new();
    btree_index.build(&amps).unwrap();

    let mut fst_index = FstAmpIndex::new();
    fst_index.build(&amps).unwrap();

    // Sample queries
    let queries = ["a", "am", "ama", "amaz", "amazon"];

    let mut group = c.benchmark_group("query");

    for query in &queries {
        group.bench_with_input(BenchmarkId::new("btree", query), query, |b, q| {
            b.iter(|| btree_index.query(q).unwrap())
        });

        group.bench_with_input(BenchmarkId::new("fst", query), query, |b, q| {
            b.iter(|| fst_index.query(q).unwrap())
        });
    }

    group.finish();
}

fn bench_memory(c: &mut Criterion) {
    // Load the AMP data
    let amps = load_amp_data("data/amp-us-desktop.json").unwrap();

    // Benchmark memory usage through stats
    let group = c.benchmark_group("memory");

    // Build each index once
    let mut btree_index = BTreeAmpIndex::new();
    btree_index.build(&amps).unwrap();
    let btree_stats = btree_index.stats();

    let mut fst_index = FstAmpIndex::new();
    fst_index.build(&amps).unwrap();
    let fst_stats = fst_index.stats();

    // Print memory stats
    println!("BTree stats: {:?}", btree_stats);
    println!("FST stats: {:?}", fst_stats);

    group.finish();
}

criterion_group!(benches, bench_build, bench_query, bench_memory);
criterion_main!(benches);
