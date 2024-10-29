use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
};

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::script::ScriptExecutionContext;

use crate::log::LogLevel;

pub async fn execute_command(command: &str, context: &mut ScriptExecutionContext<'_>) -> Result<(), String> {
    let child = if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", command]);
        cmd.current_dir(context.directory);
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?
    } else {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd.current_dir(context.directory);
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?
    };

    execute_script(child, context).await
}

pub async fn execute_command_with_env(
    command: &str,
    env: Vec<(String, String)>,
    context: &mut ScriptExecutionContext<'_>,
) -> Result<(), String> {
    let child = if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", command]).current_dir(context.directory);
        for (key, value) in env {
            cmd.env(key, value);
        }
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?
    } else {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command).current_dir(context.directory);
        for (key, value) in env {
            cmd.env(key, value);
        }
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?
    };

    execute_script(child, context).await
}

pub async fn execute_script(mut child: Child, context: &mut ScriptExecutionContext<'_>) -> Result<(), String> {
    let stdout = child.stdout.take();
    if stdout.is_none() {
        return Err("Failed to open stdout".to_string());
    }
    let stdout = stdout.unwrap();
    let stderr = child.stderr.take();
    if stderr.is_none() {
        return Err("Failed to open stderr".to_string());
    }
    let stderr = stderr.unwrap();

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    // Spawn a task to handle stdout
    let job_result_clone = context.job_result.clone();
    let stdout_handle = tokio::spawn(async move {
        for line in stdout_reader.lines().map_while(Result::ok) {
            if !line.is_empty() {
                job_result_clone.add_log(LogLevel::Info, line);
            }
        }
    });

    // Spawn a task to handle stderr
    let job_result_clone = context.job_result.clone();
    let stderr_handle = tokio::spawn(async move {
        for line in stderr_reader.lines().map_while(Result::ok) {
            if !line.is_empty() {
                job_result_clone.add_log(LogLevel::Error, line);
            }
        }
    });

    // Wait for both streams to complete or for the abort signal
    tokio::select! {
        _ = stdout_handle => {},
        _ = stderr_handle => {},
        // _ = context.abort_signal.notified() => {
        //     // Attempt to kill the child process
        //     if let Err(e) = child.kill() {
        //         context.job_result.add_log(LogLevel::Error, format!("Failed to kill process: {}", e));
        //     }
        //     return Err("Process aborted".to_string());
        // }
    }

    // Wait for the child process to complete
    let status = child.wait().map_err(|e| e.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Process exited with status: {}", status))
    }
}

type HmacSha256 = Hmac<Sha256>;
pub fn is_signature_valid(payload: &str, signature: &str, secret: &str) -> Result<bool, String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e| e.to_string())?;
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    let result = format!("sha256={}", hex::encode(result.into_bytes()));
    Ok(result == signature)
}
