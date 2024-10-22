use std::path::PathBuf;

use tempfile::NamedTempFile;

use crate::{
    credential::{Credential, CredentialType},
    utils::{execute_command, execute_command_with_env},
};

pub fn git_clone(
    url: &str,
    branch: &str,
    directory: PathBuf,
    credential_id: Option<&str>,
) -> Result<(), String> {
    if cfg!(target_os = "windows") {
        // Workaround for local
        execute_command(
            &format!("git clone -b {} {}", branch, url),
            directory.clone(),
        )?;
        Ok(())
    } else if credential_id.is_some() {
        let credential =
            Credential::get(credential_id.unwrap()).ok_or("Could not get credential")?;
        if let CredentialType::Ssh(ssh_credential) = credential.value {
            // Workaround for local
            execute_command(
                &format!("git clone -b {} {}", branch, url),
                directory.clone(),
            )?;
            let tmp_file = NamedTempFile::new().map_err(|e| e.to_string())?;
            let tmp_path = tmp_file.path();
            let _ = std::fs::write(tmp_path, &ssh_credential.private_key);

            execute_command(
                &format!("chmod 400 {}", tmp_path.display()),
                directory.clone(),
            )?;

            let env = vec![(
                "GIT_SSH_COMMAND".to_string(),
                format!("ssh -i {}", tmp_path.display()),
            )];
            execute_command_with_env(
                &format!("git clone -b {} {}", branch, url),
                directory.clone(),
                env,
            )?;

            Ok(())
        } else {
            Err("Invalid credential type".into())
        }
    } else {
        Err("Credential ID is required".into())
    }
}
