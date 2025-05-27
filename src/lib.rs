pub mod btree;
pub mod common;
pub mod blart;
pub mod hybrid;

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub use blart::BlartAmpIndex;
pub use btree::BTreeAmpIndex;
pub use common::{AmpIndexer, AmpResult, OriginalAmp};
pub use hybrid::HybridAmpIndex;

/// Utility function to load AMP data from a JSON file
pub fn load_amp_data<P: AsRef<Path>>(path: P) -> Result<Vec<OriginalAmp>, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let amps = serde_json::from_reader(reader)?;
    Ok(amps)
}
