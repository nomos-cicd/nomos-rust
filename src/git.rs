use std::path::Path;

use tempfile::NamedTempFile;

use crate::{
    credential::{Credential, CredentialType},
    job::JobResult,
    log::LogLevel,
    utils::{execute_command, execute_command_with_env},
};

pub fn git_clone(
    url: &str,
    branch: &str,
    directory: &Path,
    credential_id: Option<&str>,
    job_result: &mut JobResult,
) -> Result<(), String> {
    if cfg!(target_os = "windows") {
        if !job_result.dry_run {
            // Workaround for local
            execute_command(&format!("git clone -b {} {}", branch, url), directory, job_result)?;
        }

        Ok(())
    } else if let Some(cred_id) = credential_id {
        let credential = match Credential::get(cred_id, Some(job_result))? {
            Some(cred) => cred,
            None => return Err(format!("Credential not found: {}", cred_id)),
        };

        match credential.value {
            CredentialType::Ssh(ssh_credential) => {
                job_result.add_log(LogLevel::Info, "command: chmod 400 <private_key_temp_file>".to_string());
                job_result.add_log(LogLevel::Info, format!("command: git clone -b {} {}", branch, url));
                if !job_result.dry_run {
                    let tmp_file = NamedTempFile::new().map_err(|e| e.to_string())?;
                    let tmp_path = tmp_file.path();
                    std::fs::write(tmp_path, ssh_credential.private_key).map_err(|e| e.to_string())?;

                    execute_command(&format!("chmod 400 {}", tmp_path.display()), directory, job_result)?;

                    let env = vec![(
                        "GIT_SSH_COMMAND".to_string(),
                        format!("ssh -i {} -o StrictHostKeyChecking=no", tmp_path.display()),
                    )];
                    execute_command_with_env(&format!("git clone -b {} {}", branch, url), directory, env, job_result)?;
                }

                Ok(())
            }
            _ => Err("Invalid credential type".into()),
        }
    } else {
        Err("Credential ID is required".into())
    }
}
