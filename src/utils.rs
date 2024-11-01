use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
};

use hmac::{Hmac, Mac};
use sha2::Sha256;
use sysinfo::{Pid, System};

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

async fn execute_script(mut child: Child, context: &mut ScriptExecutionContext<'_>) -> Result<(), String> {
    eprintln!("Child process id: {}", child.id());
    context
        .job_result
        .child_process_ids
        .push(child.id().try_into().unwrap());
    context.job_result.save()?;
    let stdout = child.stdout.take();
    if stdout.is_none() {
        context.job_result.child_process_ids.pop();
        return Err("Failed to open stdout".to_string());
    }
    let stdout = stdout.unwrap();
    let stderr = child.stderr.take();
    if stderr.is_none() {
        context.job_result.child_process_ids.pop();
        return Err("Failed to open stderr".to_string());
    }
    let stderr = stderr.unwrap();

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    // Spawn a task to handle stdout
    let job_result_clone = context.job_result.clone();
    tokio::spawn(async move {
        for line in stdout_reader.lines().map_while(Result::ok) {
            if !line.is_empty() {
                job_result_clone.add_log(LogLevel::Info, line);
            }
        }
    });

    // Spawn a task to handle stderr
    let job_result_clone = context.job_result.clone();
    tokio::spawn(async move {
        for line in stderr_reader.lines().map_while(Result::ok) {
            if !line.is_empty() {
                job_result_clone.add_log(LogLevel::Error, line);
            }
        }
    });

    loop {
        let is_child_running = match child.try_wait() {
            Ok(Some(_)) => false,
            Ok(None) => true,
            Err(_) => false,
        };
        if !is_child_running {
            break;
        }

        tokio::task::yield_now().await;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    tokio::task::yield_now().await;

    let status = child.wait().map_err(|e| e.to_string())?;
    context.job_result.child_process_ids.pop();

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

pub fn get_process_recursive(pid: usize) -> Vec<Pid> {
    let s = System::new_all();
    let root_pid = Pid::from(pid);
    let processes = s.processes();

    let mut result = Vec::new();
    let mut last_list = [root_pid].to_vec();
    while !last_list.is_empty() {
        let mut new_list = Vec::new();
        for parent_pid in last_list {
            for (child_pid, process) in processes {
                if process.parent() == Some(parent_pid) {
                    new_list.push(*child_pid);
                    result.push(*child_pid);
                }
            }
        }
        last_list = new_list;
    }

    result
}
