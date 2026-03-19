//! sutra-docker — Docker/OCI container management.
//!
//! Actions:
//! - `pull` — Pull a container image
//! - `run` — Run a container (create + start)
//! - `stop` — Stop a running container
//! - `rm` — Remove a container
//! - `compose_up` — docker compose up
//! - `compose_down` — docker compose down

use sutra_core::{
    Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, esc, param_bool, param_str,
};

pub struct DockerModule;

impl DockerModule {
    async fn container_running(&self, exec: &Executor, name: &str) -> anyhow::Result<bool> {
        let result = exec
            .exec(&format!(
                "docker inspect -f '{{{{.State.Running}}}}' {} 2>/dev/null",
                esc(name)
            ))
            .await?;
        Ok(result.success() && result.stdout.trim() == "true")
    }

    async fn image_exists(&self, exec: &Executor, image: &str) -> anyhow::Result<bool> {
        let result = exec
            .exec(&format!("docker image inspect {} 2>/dev/null", esc(image)))
            .await?;
        Ok(result.success())
    }
}

impl SutraModule for DockerModule {
    fn name(&self) -> &str {
        "docker"
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
                    (false, format!("image {} already pulled", image))
                } else {
                    (true, format!("pull image {}", image))
                }
            }
            "run" => {
                let container_name = param_str(task, "container_name", "");
                let image = param_str(task, "image", "");
                if !container_name.is_empty() {
                    let running = self.container_running(exec, container_name).await?;
                    if running {
                        (
                            false,
                            format!("container {} already running", container_name),
                        )
                    } else {
                        (
                            true,
                            format!("run container {} from {}", container_name, image),
                        )
                    }
                } else {
                    (true, format!("run container from {}", image))
                }
            }
            "stop" => {
                let container_name = param_str(task, "container_name", "");
                let running = self.container_running(exec, container_name).await?;
                if running {
                    (true, format!("stop container {}", container_name))
                } else {
                    (
                        false,
                        format!("container {} already stopped", container_name),
                    )
                }
            }
            "rm" => {
                let container_name = param_str(task, "container_name", "");
                (true, format!("remove container {}", container_name))
            }
            "compose_up" => {
                let file = param_str(task, "file", "docker-compose.yml");
                (true, format!("compose up from {}", file))
            }
            "compose_down" => {
                let file = param_str(task, "file", "docker-compose.yml");
                (true, format!("compose down from {}", file))
            }
            other => anyhow::bail!("unknown docker action: {}", other),
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
                format!("docker pull {}", esc(image))
            }
            "run" => {
                let image = param_str(task, "image", "");
                let container_name = param_str(task, "container_name", "");
                let ports = param_str(task, "ports", "");
                let detach = param_bool(task, "detach", true);
                let restart = param_str(task, "restart", "");
                let env = param_str(task, "env", "");

                let mut cmd = String::from("docker run");
                if detach {
                    cmd.push_str(" -d");
                }
                if !container_name.is_empty() {
                    cmd.push_str(&format!(" --name {}", esc(container_name)));
                }
                if !ports.is_empty() {
                    cmd.push_str(&format!(" -p {}", esc(ports)));
                }
                if !restart.is_empty() {
                    cmd.push_str(&format!(" --restart {}", esc(restart)));
                }
                if !env.is_empty() {
                    cmd.push_str(&format!(" -e {}", esc(env)));
                }
                cmd.push_str(&format!(" {}", esc(image)));
                cmd
            }
            "stop" => {
                let container_name = param_str(task, "container_name", "");
                format!("docker stop {}", esc(container_name))
            }
            "rm" => {
                let container_name = param_str(task, "container_name", "");
                let force = param_bool(task, "force", false);
                if force {
                    format!("docker rm -f {}", esc(container_name))
                } else {
                    format!("docker rm {}", esc(container_name))
                }
            }
            "compose_up" => {
                let file = param_str(task, "file", "docker-compose.yml");
                let detach = param_bool(task, "detach", true);
                if detach {
                    format!("docker compose -f {} up -d", esc(file))
                } else {
                    format!("docker compose -f {} up", esc(file))
                }
            }
            "compose_down" => {
                let file = param_str(task, "file", "docker-compose.yml");
                let volumes = param_bool(task, "volumes", false);
                if volumes {
                    format!("docker compose -f {} down -v", esc(file))
                } else {
                    format!("docker compose -f {} down", esc(file))
                }
            }
            other => anyhow::bail!("unknown docker action: {}", other),
        };

        let result = exec.exec(&cmd).await?;

        Ok(TaskResult {
            module: self.name().to_string(),
            action: task.action.clone(),
            success: result.success(),
            changed: result.success(),
            message: if result.success() {
                format!("docker {} — ok", task.action)
            } else {
                format!("docker {} — failed: {}", task.action, result.stderr.trim())
            },
        })
    }

    async fn check(&self, task: &Task, _node: &NodeInfo, exec: &Executor) -> anyhow::Result<bool> {
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
    fn test_docker_actions() {
        let module = DockerModule;
        assert_eq!(module.name(), "docker");
        assert!(module.actions().contains(&"pull"));
        assert!(module.actions().contains(&"run"));
        assert!(module.actions().contains(&"compose_up"));
    }
}
