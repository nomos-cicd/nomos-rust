use std::path::PathBuf;

use crate::{job::JobResult, utils::execute_command};

/// docker run -d --restart unless-stopped {..args}
pub fn docker_run(image: &str, args: Vec<&str>, directory: PathBuf, job_result: &mut JobResult) -> Result<(), String> {
    let mut command = vec!["docker", "run", "-d"];
    command.push(image);
    command.extend(args);

    execute_command(&command.join(" "), directory, job_result)
}

/// docker build -t {image} -f {dockerfile}
pub fn docker_build(
    image: &str,
    dockerfile: PathBuf,
    directory: PathBuf,
    job_result: &mut JobResult,
) -> Result<(), String> {
    let dockerfile_dir = dockerfile.parent().unwrap();
    let command = format!(
        "docker build {} -t {} -f {}",
        dockerfile_dir.to_str().unwrap(),
        image,
        dockerfile.display()
    );
    execute_command(&command, directory, job_result)
}

/// docker stop {container} && docker rm {container}
pub fn docker_stop_and_rm(container: &str, directory: PathBuf, job_result: &mut JobResult) {
    let _ = execute_command(&format!("docker stop {}", container), directory.clone(), job_result);
    let _ = execute_command(&format!("docker rm {}", container), directory, job_result);
}
