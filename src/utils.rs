use std::{io::{BufRead, BufReader}, path::PathBuf, process::{Child, Command, Stdio}};

pub fn execute_command(command: &str, directory: PathBuf) -> Result<(), String> {
    let child = if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd");
        cmd.args(&["/C", command]);
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

    execute_script(child)
}

pub fn execute_command_with_env(command: &str, directory: PathBuf, env: Vec<(String, String)>) -> Result<(), String> {
    let child = if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd");
        cmd.args(&["/C", command])
            .current_dir(directory);
        for (key, value) in env {
            cmd.env(key, value);
        }
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?
    } else {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command)
            .current_dir(directory);
        for (key, value) in env {
            cmd.env(key, value);
        }
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?
    };

    execute_script(child)
}

pub fn execute_script(mut child: Child) -> Result<(), String> {
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    // Log stdout in real-time
    std::thread::spawn(move || {
        for line in stdout_reader.lines() {
            if let Ok(line) = line {
                eprintln!("STDOUT: {}", line);
            }
        }
    });

    // Log stderr in real-time
    std::thread::spawn(move || {
        for line in stderr_reader.lines() {
            if let Ok(line) = line {
                eprintln!("STDERR: {}", line);
            }
        }
    });

    // Wait for the child process to finish
    let status = child.wait().map_err(|e| e.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Process exited with status: {}", status))
    }
}
