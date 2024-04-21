
use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::io::Error;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use rayon::prelude::*;
use walkdir::WalkDir;

#[pymodule]
fn dupesearch(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<DuplicateFinder>()?;
    Ok(())
}

#[derive(Default)]
struct Status {
    files_found: AtomicI32,
    files_processed: AtomicI32,
    files_deleted: AtomicI32,
    finished_finding: AtomicBool,
    finished_processing: AtomicBool,
    finished_deleting: AtomicBool,
}

#[pyclass]
#[derive(Default)]
struct DuplicateFinder {
    search_path: String,
    file_formats: Option<Vec<String>>,
    status: Status,
    duplicates_found: Arc<Mutex<Vec<Vec<PathBuf>>>>,
}

#[pymethods]
impl DuplicateFinder {
    #[new]
    fn new(search_path: String, file_formats: Option<Vec<String>>) -> Self {
        Self {
            search_path,
            file_formats,
            status: Default::default(),
            ..Default::default()
        }
    }

    #[getter]
    fn get_processed_count(&self) -> PyResult<i32> {
        Ok(self.status.files_processed.load(Ordering::Relaxed))
    }

    #[getter]
    fn get_duplicates(&self) -> PyResult<Vec<Vec<PathBuf>>> {
        return match self.duplicates_found.lock() {
            Ok(duplicates) => Ok(duplicates.clone()),
            Err(_) => Err(PyRuntimeError::new_err("unexpected error"))
        }
    }

    #[getter]
    fn get_file_count(&self) -> PyResult<i32> {
        Ok(self.status.files_found.load(Ordering::Relaxed))
    }

    #[getter]
    fn get_deleted_count(&self) -> PyResult<i32> {
        Ok(self.status.files_deleted.load(Ordering::Relaxed))
    }

    #[getter]
    fn get_has_processed_files(&self) -> PyResult<bool> {
        Ok(self.status.finished_processing.load(Ordering::Relaxed))
    }

    #[getter]
    fn get_has_found_files(&self) -> PyResult<bool> {
        Ok(self.status.finished_finding.load(Ordering::Relaxed))
    }

    #[getter]
    fn get_has_finished(&self) -> PyResult<bool> {
        Ok(self.status.finished_deleting.load(Ordering::Relaxed))
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
        for group in self.duplicates_found.lock().unwrap().iter() {
             // Keep file with shortest path
            let to_keep = group
                .iter()
                .min_by_key(|x| x.to_string_lossy().len())
                .expect("Unexpected error: empty array of duplicates");

            for path in group.iter() {
                if path != to_keep && fs::remove_file(path).is_err() {
                    eprintln!("Failed to remove file {}", path.to_string_lossy())
                }
            }
            self.status.files_deleted.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }

    fn calculate_hashes(&self) -> HashMap<u64, Vec<PathBuf>> {
        let found_files = WalkDir::new(&self.search_path)
            .into_iter()
            .map(|entry| Ok(entry?.into_path()))
            .filter(|path| {
                match path {
                    Ok(p) => {
                        p.is_file() && self.is_media_file(p)
                    },
                    Err(_) => true
                }
            }).map(|p| {self.status.files_found.fetch_add(1, Ordering::Relaxed);p})
            .collect::<Vec<io::Result<_>>>();

        self.status.finished_finding.store(true, Ordering::Relaxed);

        let hashed_files = found_files
            .into_par_iter()
            .map(|file| {
                let file = file?;
                let hash = get_hash_of_file(&file)?;
                self.status.files_processed.fetch_add(1, Ordering::Relaxed);
                Ok((file, hash))
            }).collect::<Vec<io::Result<_>>>();

        self.status.finished_processing.store(true, Ordering::Relaxed);

        let mut seen = HashMap::new();
        for t in hashed_files {
            match t {
                Ok((path, hash)) => seen.entry(hash).or_insert(Vec::new()).push(path),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        seen
    }

    fn _get_duplicate_files(&self) -> Result<(), Error> {
        let files_by_hash = self.calculate_hashes();

        let mut duplicates_found = self.duplicates_found.lock().unwrap();
        for (_hash, files) in files_by_hash.into_iter() {
            if !files.is_empty() {
                duplicates_found.push(files);
            }
        }
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
fn get_hash_of_file(f_name: &Path) -> io::Result<u64> {
    let mut file = fs::File::open(f_name)?;

    let mut buffer = [0; 1048576]; // 1 MIB
    let mut digest = CRC.digest();
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break
        }
        digest.update(&buffer[0..n]);
    }
    Ok(digest.finalize())
}
