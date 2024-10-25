use std::{
    io::{BufReader, Read},
    path::PathBuf,
    process::{Child, Command, Stdio},
};

use sha2::Sha256;
use hmac::{Hmac, Mac};

use crate::{job::JobResult, log::LogLevel};

pub fn execute_command(command: &str, directory: PathBuf, job_result: &mut JobResult) -> Result<(), String> {
    job_result.add_log(LogLevel::Info, format!("command: {}", command));
    let child = if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", command]);
        cmd.current_dir(directory);
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?
    } else {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd.current_dir(directory);
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?
    };

    execute_script(child, job_result)
}

pub fn execute_command_with_env(
    command: &str,
    directory: PathBuf,
    env: Vec<(String, String)>,
    job_result: &mut JobResult,
) -> Result<(), String> {
    job_result.add_log(LogLevel::Info, format!("command: {}", command));
    let child = if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", command]).current_dir(directory);
        for (key, value) in env {
            cmd.env(key, value);
        }
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?
    } else {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command).current_dir(directory);
        for (key, value) in env {
            cmd.env(key, value);
        }
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?
    };

    execute_script(child, job_result)
}

pub fn execute_script(mut child: Child, job_result: &mut JobResult) -> Result<(), String> {
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let mut stdout_reader = BufReader::new(stdout);
    let mut stderr_reader = BufReader::new(stderr);

    // Read entire stdout
    let mut stdout_content = String::new();
    if stdout_reader.read_to_string(&mut stdout_content).is_ok() && !stdout_content.is_empty() {
        job_result.add_log(LogLevel::Info, stdout_content);
    }

    // Read entire stderr
    let mut stderr_content = String::new();
    if stderr_reader.read_to_string(&mut stderr_content).is_ok() && !stderr_content.is_empty() {
        job_result.add_log(LogLevel::Error, stderr_content);
    }

    let status = child.wait().map_err(|e| e.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Process exited with status: {}", status))
    }
}

type HmacSha256 = Hmac<Sha256>;
pub fn is_signature_valid(payload: &str, signature: &str, secret: &str) -> bool {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("Invalid key length");
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    let result = hex::encode(result.into_bytes());
    result == signature
}
