pub mod executor;
pub mod models;
pub mod parameter;
pub mod types;

use std::path::PathBuf;

pub use executor::ScriptExecutor;
pub use parameter::{ScriptParameter, ScriptParameterType};

pub fn default_scripts_location() -> PathBuf {
    let path = if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").map_err(|e| e.to_string()).unwrap();
        PathBuf::from(appdata).join("nomos").join("scripts")
    } else {
        PathBuf::from("/var/lib/nomos/scripts")
    };
    std::fs::create_dir_all(&path).map_err(|e| e.to_string()).unwrap();
    path
}
