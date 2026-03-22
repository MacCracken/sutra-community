//! sutra-stiva — Stiva container runtime management.
//!
//! Sutra module for deploying and managing OCI containers via stiva on AGNOS
//! nodes. Provides the same actions as sutra-docker but uses stiva as the
//! container runtime instead of Docker.
//!
//! Actions:
//! - `pull` — Pull a container image
//! - `run` — Run a container (create + start)
//! - `stop` — Stop a running container
//! - `rm` — Remove a container
//! - `compose_up` — stiva compose up (TOML-based)
//! - `compose_down` — stiva compose down

use sutra_core::{Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, esc, param_bool, param_str};

pub struct StivaModule;

impl StivaModule {
    async fn container_running(&self, exec: &Executor, name: &str) -> anyhow::Result<bool> {
        let result = exec
            .exec(&format!(
                "stiva ps --format json 2>/dev/null | grep -q {}",
                esc(name)
            ))
            .await?;
        Ok(result.success())
    }

    async fn image_exists(&self, exec: &Executor, image: &str) -> anyhow::Result<bool> {
        let result = exec
            .exec(&format!("stiva images --format json 2>/dev/null | grep -q {}", esc(image)))
            .await?;
        Ok(result.success())
    }
}

impl SutraModule for StivaModule {
    fn name(&self) -> &str {
        "stiva"
    }

    fn actions(&self) -> &[&str] {
        &["pull", "run", "stop", "rm", "compose_up", "compose_down"]
    }

    async fn plan(
        &self,
        task: &Task,
        _node: &NodeInfo,
        exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let (changed, description) = match task.action.as_str() {
            "pull" => {
                let image = param_str(task, "image", "");
                let exists = self.image_exists(exec, image).await?;
                if exists {
                    (false, format!("image {image} already pulled"))
                } else {
                    (true, format!("pull image {image}"))
                }
            }
            "run" => {
                let container_name = param_str(task, "container_name", "");
                let image = param_str(task, "image", "");
                if !container_name.is_empty() {
                    let running = self.container_running(exec, container_name).await?;
                    if running {
                        (false, format!("container {container_name} already running"))
                    } else {
                        (true, format!("run container {container_name} from {image}"))
                    }
                } else {
                    (true, format!("run container from {image}"))
                }
            }
            "stop" => {
                let container_name = param_str(task, "container_name", "");
                let running = self.container_running(exec, container_name).await?;
                if running {
                    (true, format!("stop container {container_name}"))
                } else {
                    (false, format!("container {container_name} already stopped"))
                }
            }
            "rm" => {
                let container_name = param_str(task, "container_name", "");
                (true, format!("remove container {container_name}"))
            }
            "compose_up" => {
                let file = param_str(task, "file", "stiva-compose.toml");
                (true, format!("compose up from {file}"))
            }
            "compose_down" => {
                let file = param_str(task, "file", "stiva-compose.toml");
                (true, format!("compose down from {file}"))
            }
            other => anyhow::bail!("unknown stiva action: {other}"),
        };

        Ok(TaskPlan {
            module: self.name().to_string(),
            action: task.action.clone(),
            changed,
            description,
            diff: None,
        })
    }

    async fn apply(
        &self,
        task: &Task,
        _node: &NodeInfo,
        exec: &Executor,
    ) -> anyhow::Result<TaskResult> {
        let cmd = match task.action.as_str() {
            "pull" => {
                let image = param_str(task, "image", "");
                format!("stiva pull {}", esc(image))
            }
            "run" => {
                let image = param_str(task, "image", "");
                let container_name = param_str(task, "container_name", "");
                let ports = param_str(task, "ports", "");
                let env = param_str(task, "env", "");
                let restart = param_str(task, "restart", "");

                let mut cmd = String::from("stiva run");
                if !container_name.is_empty() {
                    cmd.push_str(&format!(" --name {}", esc(container_name)));
                }
                if !ports.is_empty() {
                    cmd.push_str(&format!(" -p {}", esc(ports)));
                }
                if !env.is_empty() {
                    cmd.push_str(&format!(" -e {}", esc(env)));
                }
                if !restart.is_empty() {
                    cmd.push_str(&format!(" --restart {}", esc(restart)));
                }
                cmd.push_str(&format!(" {}", esc(image)));
                cmd
            }
            "stop" => {
                let container_name = param_str(task, "container_name", "");
                format!("stiva stop {}", esc(container_name))
            }
            "rm" => {
                let container_name = param_str(task, "container_name", "");
                let force = param_bool(task, "force", false);
                if force {
                    format!("stiva rm -f {}", esc(container_name))
                } else {
                    format!("stiva rm {}", esc(container_name))
                }
            }
            "compose_up" => {
                let file = param_str(task, "file", "stiva-compose.toml");
                format!("stiva compose up -f {}", esc(file))
            }
            "compose_down" => {
                let file = param_str(task, "file", "stiva-compose.toml");
                let volumes = param_bool(task, "volumes", false);
                if volumes {
                    format!("stiva compose down -f {} --volumes", esc(file))
                } else {
                    format!("stiva compose down -f {}", esc(file))
                }
            }
            other => anyhow::bail!("unknown stiva action: {other}"),
        };

        let result = exec.exec(&cmd).await?;

        Ok(TaskResult {
            module: self.name().to_string(),
            action: task.action.clone(),
            success: result.success(),
            changed: result.success(),
            message: if result.success() {
                format!("stiva {} — ok", task.action)
            } else {
                format!(
                    "stiva {} — failed: {}",
                    task.action,
                    result.stderr.trim()
                )
            },
        })
    }

    async fn check(
        &self,
        task: &Task,
        _node: &NodeInfo,
        exec: &Executor,
    ) -> anyhow::Result<bool> {
        match task.action.as_str() {
            "pull" => {
                let image = param_str(task, "image", "");
                self.image_exists(exec, image).await
            }
            "run" => {
                let container_name = param_str(task, "container_name", "");
                if container_name.is_empty() {
                    return Ok(false);
                }
                self.container_running(exec, container_name).await
            }
            "stop" => {
                let container_name = param_str(task, "container_name", "");
                self.container_running(exec, container_name)
                    .await
                    .map(|v| !v)
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stiva_actions() {
        let module = StivaModule;
        assert_eq!(module.name(), "stiva");
        assert!(module.actions().contains(&"pull"));
        assert!(module.actions().contains(&"run"));
        assert!(module.actions().contains(&"stop"));
        assert!(module.actions().contains(&"rm"));
        assert!(module.actions().contains(&"compose_up"));
        assert!(module.actions().contains(&"compose_down"));
    }

    #[test]
    fn test_actions_count() {
        let module = StivaModule;
        assert_eq!(module.actions().len(), 6);
    }
}
