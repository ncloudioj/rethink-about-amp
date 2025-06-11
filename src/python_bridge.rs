use pyo3::exceptions::{PyIOError, PyKeyError, PyValueError};
use pyo3::prelude::*;
use serde_json;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::{AmpIndexer, AmpResult, BlartAmpIndex, OriginalAmp};

#[pyclass]
#[derive(Clone)]
pub struct PyAmpResult {
    #[pyo3(get)]
    pub title: String,
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub click_url: String,
    #[pyo3(get)]
    pub impression_url: String,
    #[pyo3(get)]
    pub advertiser: String,
    #[pyo3(get)]
    pub block_id: i32,
    #[pyo3(get)]
    pub iab_category: String,
    #[pyo3(get)]
    pub icon: String,
    #[pyo3(get)]
    pub full_keyword: String,
}

impl From<AmpResult> for PyAmpResult {
    fn from(result: AmpResult) -> Self {
        PyAmpResult {
            title: result.title,
            url: result.url,
            click_url: result.click_url,
            impression_url: result.impression_url,
            advertiser: result.advertiser,
            block_id: result.block_id,
            iab_category: result.iab_category,
            icon: result.icon,
            full_keyword: result.full_keyword,
        }
    }
}

// Thread-safe index wrapper
type IndexHandle = Arc<RwLock<BlartAmpIndex>>;

#[pyclass]
pub struct AmpIndexManager {
    indexes: Arc<RwLock<HashMap<String, IndexHandle>>>,
}

#[pymethods]
impl AmpIndexManager {
    #[new]
    fn new() -> PyResult<Self> {
        Ok(AmpIndexManager {
            indexes: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Build index from JSON file
    fn build_from_file(&self, index_name: String, json_path: String) -> PyResult<()> {
        let amps = crate::load_amp_data(&json_path)
            .map_err(|e| PyIOError::new_err(format!("Failed to load JSON: {}", e)))?;

        let mut index = BlartAmpIndex::new();
        index
            .build(&amps)
            .map_err(|e| PyValueError::new_err(format!("Failed to build index: {}", e)))?;

        let mut indexes = self.indexes.write().unwrap();
        indexes.insert(index_name, Arc::new(RwLock::new(index)));
        Ok(())
    }

    /// Build index from JSON string
    fn build_from_json(&self, index_name: String, json_data: String) -> PyResult<()> {
        let amps: Vec<OriginalAmp> = serde_json::from_str(&json_data)
            .map_err(|e| PyValueError::new_err(format!("Invalid JSON: {}", e)))?;

        let mut index = BlartAmpIndex::new();
        index
            .build(&amps)
            .map_err(|e| PyValueError::new_err(format!("Failed to build index: {}", e)))?;

        let mut indexes = self.indexes.write().unwrap();
        indexes.insert(index_name, Arc::new(RwLock::new(index)));
        Ok(())
    }

    /// Query index
    fn query(&self, index_name: String, query: String) -> PyResult<Vec<PyAmpResult>> {
        let indexes = self.indexes.read().unwrap();
        let index_handle = indexes
            .get(&index_name)
            .ok_or_else(|| PyKeyError::new_err(format!("Index '{}' not found", index_name)))?;

        let index = index_handle.read().unwrap();
        let results = index
            .query(&query)
            .map_err(|e| PyValueError::new_err(format!("Query failed: {}", e)))?;

        Ok(results.into_iter().map(PyAmpResult::from).collect())
    }

    /// Delete index
    fn delete(&self, index_name: String) -> PyResult<()> {
        let mut indexes = self.indexes.write().unwrap();
        indexes
            .remove(&index_name)
            .ok_or_else(|| PyKeyError::new_err(format!("Index '{}' not found", index_name)))?;
        Ok(())
    }

    /// List indexes
    fn list(&self) -> Vec<String> {
        let indexes = self.indexes.read().unwrap();
        indexes.keys().cloned().collect()
    }

    /// Check if index exists
    fn has(&self, index_name: String) -> bool {
        let indexes = self.indexes.read().unwrap();
        indexes.contains_key(&index_name)
    }
}

// Module registration
pub fn register_module(m: &PyModule) -> PyResult<()> {
    m.add_class::<PyAmpResult>()?;
    m.add_class::<AmpIndexManager>()?;
    Ok(())
}
