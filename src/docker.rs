use std::path::PathBuf;

use crate::{job::JobResult, log::LogLevel, utils::execute_command};

/// docker run -d {..args}
pub fn docker_run(image: &str, args: Vec<&str>, directory: PathBuf, job_result: &mut JobResult) -> Result<(), String> {
    let mut command = vec!["docker", "run", "-d"];
    command.extend(args);
    command.push(image);

    job_result.add_log(LogLevel::Info, format!("command: docker run -d <args> {}", image));
    if !job_result.dry_run {
        execute_command(&command.join(" "), directory, job_result)?;
    }
    Ok(())
}

/// docker build -t {image} -f {dockerfile}
pub fn docker_build(
    image: &str,
    dockerfile: PathBuf,
    directory: PathBuf,
    job_result: &mut JobResult,
) -> Result<(), String> {
    let dockerfile_dir = dockerfile.parent();
    if dockerfile_dir.is_none() {
        return Err("Dockerfile directory not found".to_string());
    }
    let dockerfile_dir = dockerfile_dir.unwrap();
    let command = format!(
        "docker build {} -t {} -f {}",
        dockerfile_dir.to_str().unwrap(),
        image,
        dockerfile.display()
    );
    job_result.add_log(LogLevel::Info, format!("command: {}", command));
    if !job_result.dry_run {
        execute_command(&command, directory, job_result)?;
    }
    Ok(())
}

/// docker stop {container} && docker rm {container}
pub fn docker_stop_and_rm(container: &str, directory: PathBuf, job_result: &mut JobResult) {
    job_result.add_log(LogLevel::Info, format!("command: docker stop {}", container));
    if !job_result.dry_run {
        let _ = execute_command(&format!("docker stop {}", container), directory.clone(), job_result);
    }
    job_result.add_log(LogLevel::Info, format!("command: docker rm {}", container));
    if !job_result.dry_run {
        let _ = execute_command(&format!("docker rm {}", container), directory, job_result);
    }
}
