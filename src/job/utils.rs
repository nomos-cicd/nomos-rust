use once_cell::sync::Lazy;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use super::models::JobResult;

pub fn default_job_results_location() -> Result<PathBuf, String> {
    let path = if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").map_err(|e| e.to_string())?;
        PathBuf::from(appdata).join("nomos").join("job_results")
    } else {
        PathBuf::from("/var/lib/nomos/job_results")
    };
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn default_jobs_location() -> Result<PathBuf, String> {
    let path = if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").map_err(|e| e.to_string())?;
        PathBuf::from(appdata).join("nomos").join("jobs")
    } else {
        PathBuf::from("/var/lib/nomos/jobs")
    };
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path)
}

static JOB_RESULTS: Lazy<Arc<Mutex<File>>> = Lazy::new(|| {
    let path = if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
        let mut path = PathBuf::from(appdata);
        path.push("nomos");
        path.push("ids.txt");
        path
    } else {
        let mut path = PathBuf::from("/var/lib/nomos");
        path.push("ids.txt");
        path
    };

    // Ensure the parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).expect("Failed to create directories");
        }
    }

    // Open the file with read/write permissions, create if it doesn't exist
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .expect("Failed to open or create file");

    Arc::new(Mutex::new(file))
});

/// Reads .../nomos/ids.txt and returns the next job id
pub fn next_job_result_id() -> Result<String, String> {
    let binding = Arc::clone(&JOB_RESULTS);
    let mut file = binding.lock().unwrap_or_else(|e| e.into_inner());

    let mut content = String::new();
    file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    file.read_to_string(&mut content).map_err(|e| e.to_string())?;

    let id = content.trim().parse::<u64>().unwrap_or(0);

    let mut next_id = id + 1;
    while JobResult::get(&next_id.to_string())?.is_some() {
        next_id += 1;
    }

    file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    file.set_len(0).map_err(|e| e.to_string())?;
    file.write_all(next_id.to_string().as_bytes())
        .map_err(|e| e.to_string())?;
    file.flush().map_err(|e| e.to_string())?;

    Ok(next_id.to_string())
}
