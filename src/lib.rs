use md5;
use pyo3::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::io::prelude::*;
use std::io::Error;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
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

    files_found_counter: Arc<Mutex<i32>>,
    files_processed_counter: Arc<Mutex<i32>>,
    deleted_files_counter: Arc<Mutex<i32>>,

    files_found: Arc<Mutex<Vec<String>>>,
    duplicates_found: Arc<Mutex<Vec<Vec<String>>>>,

    finished_finding_files: Arc<AtomicBool>,
    finished_processing_files: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,

    file_formats: Option<Vec<String>>
}

#[pymethods]
impl DuplicateFinder {
    #[new]
    fn new(search_path: String, file_formats: Option<Vec<String>>) -> Self {
        let files_found_counter = Arc::new(Mutex::new(0));
        let files_processed_counter = Arc::new(Mutex::new(0));
        let deleted_files_counter = Arc::new(Mutex::new(0));

        let files_found = Arc::new(Mutex::new(Vec::new()));
        let duplicates_found = Arc::new(Mutex::new(Vec::new()));

        let finished_processing_files = Arc::new(AtomicBool::new(false));
        let finished_finding_files = Arc::new(AtomicBool::new(false));
        let finished = Arc::new(AtomicBool::new(false));


        DuplicateFinder {
            files_found_counter,
            files_processed_counter,
            deleted_files_counter,
            files_found,
            duplicates_found,
            finished_processing_files,
            finished_finding_files,
            finished,
            search_path,
            file_formats,
        }
    }
    #[getter]
    fn get_processed_count(&self) -> PyResult<i32> {
        Ok(*self.files_processed_counter.lock().unwrap())
    }

    #[getter]
    fn get_duplicates(&self) -> PyResult<Vec<Vec<String>>> {
        Ok(self.duplicates_found.lock().expect("").to_vec())
    }

    #[getter]
    fn get_file_count(&self) -> PyResult<i32> {
        Ok(*self.files_found_counter.lock().unwrap())
    }

    #[getter]
    fn get_deleted_count(&self) -> PyResult<i32> {
        Ok(*self.deleted_files_counter.lock().unwrap())
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
        for group in self.duplicates_found.lock().unwrap().iter() {
            let to_keep = group.iter().min_by_key(|x| x.len()).unwrap(); // Keep shortest path name
            for item in group.iter() {
                if item != to_keep {
                    fs::remove_file(item)?;
                }
            }
            *counter.lock().unwrap() += 1;
        }
        Ok(())
    }

    fn find_files_to_search(&self) {
        let counter = Arc::clone(&self.files_found_counter);
        for entry in WalkDir::new(&self.search_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| !e.path().is_dir() & self.is_media_file(e.path()))
        {
            self.files_found
                .lock()
                .unwrap()
                .push(entry.path().to_str().unwrap().to_string());
            *counter.lock().unwrap() += 1;
        }
        self.finished_finding_files.store(true, Ordering::Relaxed);
    }

    fn calculate_hashes(&self) -> HashMap<md5::Digest, Vec<String>> {
        let pool = ThreadPool::new(8);
        let (tx, rx) = mpsc::channel();
        for file in self.files_found.lock().unwrap().iter() {
            let tx = tx.clone();
            let counter = Arc::clone(&self.files_processed_counter);
            let file = file.clone();
            pool.execute(move || {
                let buffer = read_sample_of_file(Path::new(&file)).unwrap();
                tx.send((file, md5::compute(buffer))).unwrap();
                *counter.lock().unwrap() += 1;
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

        for (_hash, files) in files_by_hash.iter() {
            if files.len() > 1 {
                self.duplicates_found.lock().unwrap().push(files.clone());
            }
        }
        self.finished.store(true, Ordering::Relaxed);
        Ok(())
    }

    fn is_media_file(&self, f_name: &Path) -> bool {
        match &self.file_formats{
            Some(file_formats) => file_formats.contains(
                &f_name
                    .extension()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap()
                    .to_lowercase()
            ),
            None => true
        }
    }
}

fn read_sample_of_file(f_name: &Path) -> Result<Vec<u8>, Error> {
    let mut file = fs::File::open(f_name)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}
