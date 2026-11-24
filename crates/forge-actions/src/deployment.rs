//! SSH-based Docker Compose deployment and rollback actions.

use crate::{check_output, validate_non_empty, ActionError, ActionOutput};
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct DeploymentEnvironment {
    pub name: String,
    pub host: String,
    pub user: String,
    pub port: u16,
    pub identity_file: Option<PathBuf>,
    pub working_directory: String,
    pub compose_file: String,
    pub service: String,
    pub image_variable: String,
}

#[derive(Debug, Clone)]
pub struct ComposeDeployment {
    pub environment: DeploymentEnvironment,
    pub image: String,
}

impl ComposeDeployment {
    pub async fn deploy(&self) -> Result<ActionOutput, ActionError> {
        self.run(&self.image).await
    }

    /// Re-deploys a previously recorded image reference using the same
    /// environment and Compose service contract as a forward deployment.
    pub async fn rollback(&self, previous_image: &str) -> Result<ActionOutput, ActionError> {
        self.run(previous_image).await
    }

    async fn run(&self, image: &str) -> Result<ActionOutput, ActionError> {
        let env = &self.environment;
        for (name, value) in [
            ("environment name", env.name.as_str()),
            ("host", env.host.as_str()),
            ("user", env.user.as_str()),
            ("working directory", env.working_directory.as_str()),
            ("Compose file", env.compose_file.as_str()),
            ("service", env.service.as_str()),
            ("image variable", env.image_variable.as_str()),
            ("image", image),
        ] {
            validate_non_empty(name, value)?;
        }
        if !env
            .image_variable
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(ActionError::InvalidInput(
                "image variable must contain only ASCII letters, digits, and underscores".into(),
            ));
        }

        let destination = format!("{}@{}", env.user, env.host);
        if !env
            .host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-:[]".contains(c))
            || !env
                .user
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
        {
            return Err(ActionError::InvalidInput(
                "SSH user and host contain unsupported characters".into(),
            ));
        }
        let remote = format!(
            "cd {} && export {}={} && docker compose --file {} pull {} && docker compose --file {} up --detach {}",
            shell_quote(&env.working_directory),
            env.image_variable,
            shell_quote(image),
            shell_quote(&env.compose_file),
            shell_quote(&env.service),
            shell_quote(&env.compose_file),
            shell_quote(&env.service),
        );
        let mut command = Command::new("ssh");
        command
            .arg("-p")
            .arg(env.port.to_string())
            .args(["-o", "BatchMode=yes"]);
        if let Some(identity) = &env.identity_file {
            command.arg("-i").arg(identity);
        }
        let output = command
            .arg(destination)
            .arg(format!("sh -lc {}", shell_quote(&remote)))
            .output()
            .await
            .map_err(|source| ActionError::Start {
                program: "ssh docker compose deployment".into(),
                source,
            })?;
        check_output("ssh docker compose deployment", output)
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
