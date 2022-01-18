use crc;
use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use std::collections::HashMap;
use std::fs;
use std::io::prelude::*;
use std::io::Error;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use threadpool::ThreadPool;
use walkdir::WalkDir;

#[pymodule]
fn dupesearch(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<DuplicateFinder>()?;
    Ok(())
}

#[pyclass]
struct DuplicateFinder {
    search_path: String,
    file_formats: Option<Vec<String>>,

    files_found_counter: Arc<AtomicI32>,
    files_processed_counter: Arc<AtomicI32>,
    deleted_files_counter: Arc<AtomicI32>,

    finished_finding_files: Arc<AtomicBool>,
    finished_processing_files: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,

    files_found: Arc<Mutex<Vec<PathBuf>>>,
    duplicates_found: Arc<Mutex<Vec<Vec<PathBuf>>>>,
}

#[pymethods]
impl DuplicateFinder {
    #[new]
    fn new(search_path: String, file_formats: Option<Vec<String>>) -> Self {
        DuplicateFinder {
            search_path,
            file_formats,

            files_found_counter: Arc::new(AtomicI32::new(0)),
            files_processed_counter: Arc::new(AtomicI32::new(0)),
            deleted_files_counter: Arc::new(AtomicI32::new(0)),

            finished_processing_files: Arc::new(AtomicBool::new(false)),
            finished_finding_files: Arc::new(AtomicBool::new(false)),
            finished: Arc::new(AtomicBool::new(false)),

            files_found: Arc::new(Mutex::new(Vec::new())),
            duplicates_found: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[getter]
    fn get_processed_count(&self) -> PyResult<i32> {
        return Ok(self.files_processed_counter.load(Ordering::Relaxed))
    }

    #[getter]
    fn get_duplicates(&self) -> PyResult<Vec<Vec<String>>> {
        return match self.duplicates_found.lock(){
            Ok(n) => Ok(
                n.clone().into_iter()
                .map(|x|
                    x.clone().into_iter()
                    .map(|path| path.to_string_lossy().into_owned()).collect()
                ).collect()
            ),
            Err(_) => Err(PyRuntimeError::new_err("unexpected error")),
        };
    }

    #[getter]
    fn get_file_count(&self) -> PyResult<i32> {
        Ok(self.files_found_counter.load(Ordering::Relaxed))
    }

    #[getter]
    fn get_deleted_count(&self) -> PyResult<i32> {
        Ok(self.deleted_files_counter.load(Ordering::Relaxed))
    }

    #[getter]
    fn get_has_processed_files(&self) -> PyResult<bool> {
        Ok(self.finished_processing_files.load(Ordering::Relaxed))
    }

    #[getter]
    fn get_has_found_files(&self) -> PyResult<bool> {
        Ok(self.finished_finding_files.load(Ordering::Relaxed))
    }

    #[getter]
    fn get_has_finished(&self) -> PyResult<bool> {
        Ok(self.finished.load(Ordering::Relaxed))
    }

    fn find_duplicates(&self, py: Python) -> PyResult<()> {
        py.allow_threads(|| self._get_duplicate_files())?;
        Ok(())
    }

    fn delete_duplicates(&self, py: Python) -> PyResult<()> {
        py.allow_threads(|| self._delete_duplicates())?;
        Ok(())
    }
}

impl DuplicateFinder {
    fn _delete_duplicates(&self) -> Result<(), Error> {
        let counter = Arc::clone(&self.deleted_files_counter);
        let duplicates = self.duplicates_found.lock().unwrap();
        for group in duplicates.iter() {
             // Keep file with shortest path
            let to_keep = group
                .iter()
                .min_by_key(|x| x.to_string_lossy().len())
                .expect("Unexpected error: empty array of duplicates");

            for path in group.iter() {
                if path != to_keep {
                    if fs::remove_file(path).is_err() {
                        eprintln!("Failed to remove file {}", path.to_string_lossy())
                    };
                }
            }
            counter.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }

    fn find_files_to_search(&self) {
        let counter = Arc::clone(&self.files_found_counter);
        let mut files = self.files_found.lock().unwrap();
        for entry in WalkDir::new(&self.search_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.path().to_path_buf())
            .filter(|e| !e.is_dir() & self.is_media_file(e))
        {
            files.push(entry);
            counter.fetch_add(1, Ordering::Relaxed);
        }
        self.finished_finding_files.store(true, Ordering::Relaxed);
    }

    fn calculate_hashes(&self) -> HashMap<u64, Vec<PathBuf>> {
        let pool = ThreadPool::new(8);
        let (tx, rx) = mpsc::channel();
        for file in self.files_found.lock().unwrap().iter() {
            let tx = tx.clone();
            let counter = Arc::clone(&self.files_processed_counter);
            let file = file.clone();
            pool.execute(move || {
                if let Some(hash) = get_hash_of_file(&file){
                    tx.send((file, hash)).unwrap();
                }
                counter.fetch_add(1, Ordering::Relaxed);
            });
        }
        drop(tx);

        let mut seen = HashMap::new();
        for t in rx.iter() {
            let (path, hash) = t;
            seen.entry(hash).or_insert(Vec::new()).push(path);
        }
        self.finished_processing_files
            .store(true, Ordering::Relaxed);
        return seen;
    }

    fn _get_duplicate_files(&self) -> Result<(), Error> {
        self.find_files_to_search();

        let files_by_hash = self.calculate_hashes();

        let mut vec = self.duplicates_found.lock().unwrap();
        for (_hash, files) in files_by_hash.into_iter() {
            if files.len() > 1 {
                vec.push(files);
            }
        }
        self.finished.store(true, Ordering::Relaxed);
        Ok(())
    }

    fn is_media_file(&self, f_name: &Path) -> bool {
        if let Some(file_formats) = &self.file_formats {
            if let Some(extension) = f_name.extension() {
                if let Some(file_str) = extension.to_str() {
                    file_formats.contains(&file_str.to_lowercase())
                } else {false}
            } else {false}
        } else {true}
    }
}

const CRC: crc::Crc<u64> = crc::Crc::<u64>::new(&crc::CRC_64_ECMA_182);
fn get_hash_of_file(f_name: &Path) -> Option<u64> {
    if let Ok(mut file) = fs::File::open(f_name) {
        let mut buffer = [0; 1048576]; // 1 MIB
        let mut digest = CRC.digest();
        loop {
            if let Ok(n) = file.read(&mut buffer) {
                if n == 0 {
                    break
                }
                digest.update(&buffer[0..n]);
            } else {
                return None
            }
        }
        return Some(digest.finalize())
    }
    return None
}
