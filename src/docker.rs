use std::path::Path;

use crate::script::ScriptExecutionContext;

use crate::{log::LogLevel, utils::execute_command};

/// docker run -d {..args}
pub fn docker_run(image: &str, args: Vec<&str>, context: &mut ScriptExecutionContext<'_>) -> Result<(), String> {
    let mut command = vec!["docker", "run", "-d"];
    command.extend(args);
    command.push(image);

    context
        .job_result
        .add_log(LogLevel::Info, format!("command: docker run -d <args> {}", image));
    if !context.job_result.dry_run {
        execute_command(&command.join(" "), context)?;
    }
    Ok(())
}

/// docker build -t {image} -f {dockerfile}
pub fn docker_build(
    image: &str,
    dockerfile: &Path,
    context: &mut ScriptExecutionContext<'_>,
) -> Result<(), String> {
    let dockerfile_dir = match dockerfile.parent() {
        Some(dir) => dir,
        None => return Err("Dockerfile directory not found".to_string()),
    };

    let dockerfile_dir_str = match dockerfile_dir.to_str() {
        Some(dir_str) => dir_str,
        None => return Err("Failed to convert Dockerfile directory to string".to_string()),
    };

    let command = format!(
        "docker build {} -t {} -f {}",
        dockerfile_dir_str,
        image,
        dockerfile.display()
    );
    context
        .job_result
        .add_log(LogLevel::Info, format!("command: {}", command));
    if !context.job_result.dry_run {
        execute_command(&command, context);
    }
    Ok(())
}

/// docker stop {container} && docker rm {container}
pub fn docker_stop_and_rm(container: &str, context: &mut ScriptExecutionContext<'_>) {
    context
        .job_result
        .add_log(LogLevel::Info, format!("command: docker stop {}", container));
    if !context.job_result.dry_run {
        let _ = execute_command(&format!("docker stop {}", container), context);
    }
    context
        .job_result
        .add_log(LogLevel::Info, format!("command: docker rm {}", container));
    if !context.job_result.dry_run {
        let _ = execute_command(&format!("docker rm {}", container), context);
    }
}
