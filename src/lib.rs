pub mod blart;
pub mod btree;
pub mod common;
pub mod hybrid;

#[cfg(feature = "python")]
pub mod python_bridge;

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

// PyO3 module export - only when building as Python extension
#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pymodule]
fn rethink_about_amp(_py: Python, m: &PyModule) -> PyResult<()> {
    python_bridge::register_module(m)
}
