pub mod executor;
pub mod models;
pub mod parameter;
pub mod types;
pub mod utils;

use std::path::PathBuf;

pub use executor::*;
pub use parameter::*;

pub fn default_scripts_location() -> Result<PathBuf, String> {
    let path = if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").map_err(|e| e.to_string())?;
        PathBuf::from(appdata).join("nomos").join("scripts")
    } else {
        PathBuf::from("/var/lib/nomos/scripts")
    };
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path)
}
