use jemalloc_ctl::{epoch, stats};
use jemallocator::Jemalloc;
use rethink_about_amp::*;
use std::time::Instant;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

fn measure_memory<F, T>(name: &str, build_fn: F) -> T
where
    F: FnOnce() -> T,
{
    // Force garbage collection by dropping a large allocation
    drop(Vec::<u8>::with_capacity(1024 * 1024));

    // Update stats
    epoch::advance().unwrap();
    let start_allocated = stats::allocated::read().unwrap();
    let start_time = Instant::now();

    // Run the build function
    let result = build_fn();

    // Measure after
    let build_time = start_time.elapsed();
    epoch::advance().unwrap();
    let end_allocated = stats::allocated::read().unwrap();

    println!(
        "{} memory: {} bytes (built in: {:?})",
        name,
        end_allocated - start_allocated,
        build_time
    );

    // Return the result so the structure stays alive until we're done measuring
    result
}

fn main() {
    // Load data once
    println!("Loading AMP data...");
    let amps = load_amp_data("data/amp-us-desktop.json").unwrap();
    println!("Loaded {} AMP suggestions", amps.len());

    // Measure all structures one by one
    // This prevents interference between measurements

    // 1. BTreeMap
    {
        let index = measure_memory("BTreeMap", || {
            let mut index = BTreeAmpIndex::new();
            index.build(&amps).unwrap();
            index
        });

        // Print stats and test a query
        println!("BTree stats: {:?}", index.stats());
        let results = index.query("amaz").unwrap();
        println!("BTree query 'amaz' returned {} results", results.len());
    }

    // Add a pause between measurements
    std::thread::sleep(std::time::Duration::from_secs(1));
    println!("\n---------------------------------------\n");

    // 2. FST
    {
        let index = measure_memory("FST", || {
            let mut index = FstAmpIndex::new();
            index.build(&amps).unwrap();
            index
        });

        println!("FST stats: {:?}", index.stats());
        let results = index.query("amaz").unwrap();
        println!("FST query 'amaz' returned {} results", results.len());
    }

    std::thread::sleep(std::time::Duration::from_secs(1));
    println!("\n---------------------------------------\n");

    // Print a summary at the end
    println!("\n=========== Memory Usage Summary ===========");
    println!("Note: These measurements include all data structures,");
    println!("      not just the keyword index part.");
}
