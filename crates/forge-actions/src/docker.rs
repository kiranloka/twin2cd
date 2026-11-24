//! Docker image build, push, and Amazon ECR authentication actions.

use crate::{check_output, validate_non_empty, ActionError, ActionOutput};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct DockerBuild {
    pub context: PathBuf,
    pub image: String,
    pub dockerfile: Option<PathBuf>,
    pub build_args: Vec<(String, String)>,
    pub push: bool,
}

impl DockerBuild {
    pub async fn execute(&self) -> Result<ActionOutput, ActionError> {
        validate_non_empty("image", &self.image)?;
        if !self.context.is_dir() {
            return Err(ActionError::InvalidInput(format!(
                "build context '{}' is not a directory",
                self.context.display()
            )));
        }

        let mut command = Command::new("docker");
        command.arg("build").arg("--tag").arg(&self.image);
        if let Some(dockerfile) = &self.dockerfile {
            command.arg("--file").arg(dockerfile);
        }
        for (key, value) in &self.build_args {
            validate_non_empty("build argument name", key)?;
            command.arg("--build-arg").arg(format!("{key}={value}"));
        }
        let output = command
            .arg(&self.context)
            .output()
            .await
            .map_err(|source| ActionError::Start {
                program: "docker build".into(),
                source,
            })?;
        let mut result = check_output("docker build", output)?;

        if self.push {
            let output = Command::new("docker")
                .arg("push")
                .arg(&self.image)
                .output()
                .await
                .map_err(|source| ActionError::Start {
                    program: "docker push".into(),
                    source,
                })?;
            let pushed = check_output("docker push", output)?;
            result.stdout.push_str(&pushed.stdout);
            result.stderr.push_str(&pushed.stderr);
        }
        Ok(result)
    }
}

#[derive(Debug, Clone)]
pub struct EcrLogin {
    pub registry: String,
    pub region: String,
}

impl EcrLogin {
    /// Uses the AWS CLI credential chain and pipes the short-lived token to
    /// Docker over stdin; the token is never included in arguments or logs.
    pub async fn execute(&self) -> Result<(), ActionError> {
        validate_non_empty("ECR registry", &self.registry)?;
        validate_non_empty("AWS region", &self.region)?;
        let password = Command::new("aws")
            .args(["ecr", "get-login-password", "--region", &self.region])
            .output()
            .await
            .map_err(|source| ActionError::Start {
                program: "aws ecr get-login-password".into(),
                source,
            })?;
        let password = check_output("aws ecr get-login-password", password)?.stdout;
        if password.trim().is_empty() {
            return Err(ActionError::InvalidInput(
                "AWS CLI returned an empty ECR password".into(),
            ));
        }

        let mut child = Command::new("docker")
            .args([
                "login",
                "--username",
                "AWS",
                "--password-stdin",
                &self.registry,
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|source| ActionError::Start {
                program: "docker login".into(),
                source,
            })?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(password.as_bytes()).await?;
        }
        let output = child.wait_with_output().await?;
        check_output("docker login", output)?;
        Ok(())
    }
}
