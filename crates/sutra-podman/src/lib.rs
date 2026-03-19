//! sutra-podman — Podman rootless container management.
//!
//! Actions:
//! - `pull` — Pull a container image
//! - `run` — Run a container
//! - `stop` — Stop a running container
//! - `rm` — Remove a container
//! - `pod_create` — Create a pod
//! - `pod_start` — Start a pod
//! - `pod_stop` — Stop a pod
//! - `pod_rm` — Remove a pod
//! - `generate_systemd` — Generate systemd unit files for a container/pod

use sutra_core::{
    Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, esc, param_bool, param_str,
};

pub struct PodmanModule;

impl PodmanModule {
    async fn container_running(&self, exec: &Executor, name: &str) -> anyhow::Result<bool> {
        let result = exec
            .exec(&format!(
                "podman inspect -f '{{{{.State.Running}}}}' {} 2>/dev/null",
                esc(name)
            ))
            .await?;
        Ok(result.success() && result.stdout.trim() == "true")
    }

    async fn container_exists(&self, exec: &Executor, name: &str) -> anyhow::Result<bool> {
        let result = exec
            .exec(&format!(
                "podman container exists {} 2>/dev/null",
                esc(name)
            ))
            .await?;
        Ok(result.success())
    }

    async fn pod_exists(&self, exec: &Executor, name: &str) -> anyhow::Result<bool> {
        let result = exec
            .exec(&format!("podman pod exists {} 2>/dev/null", esc(name)))
            .await?;
        Ok(result.success())
    }

    async fn image_exists(&self, exec: &Executor, image: &str) -> anyhow::Result<bool> {
        let result = exec
            .exec(&format!("podman image exists {} 2>/dev/null", esc(image)))
            .await?;
        Ok(result.success())
    }
}

impl SutraModule for PodmanModule {
    fn name(&self) -> &str {
        "podman"
    }

    fn actions(&self) -> &[&str] {
        &[
            "pull",
            "run",
            "stop",
            "rm",
            "pod_create",
            "pod_start",
            "pod_stop",
            "pod_rm",
            "generate_systemd",
        ]
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
                    (false, format!("image {} already present", image))
                } else {
                    (true, format!("pull image {}", image))
                }
            }
            "run" => {
                let name = param_str(task, "container_name", "");
                let image = param_str(task, "image", "");
                if !name.is_empty() {
                    let running = self.container_running(exec, name).await?;
                    if running {
                        (false, format!("container {} already running", name))
                    } else {
                        (true, format!("run container {} from {}", name, image))
                    }
                } else {
                    (true, format!("run container from {}", image))
                }
            }
            "stop" => {
                let name = param_str(task, "container_name", "");
                let running = self.container_running(exec, name).await?;
                if running {
                    (true, format!("stop container {}", name))
                } else {
                    (false, format!("container {} already stopped", name))
                }
            }
            "rm" => {
                let name = param_str(task, "container_name", "");
                let exists = self.container_exists(exec, name).await?;
                if exists {
                    (true, format!("remove container {}", name))
                } else {
                    (false, format!("container {} does not exist", name))
                }
            }
            "pod_create" => {
                let name = param_str(task, "pod_name", "");
                let exists = self.pod_exists(exec, name).await?;
                if exists {
                    (false, format!("pod {} already exists", name))
                } else {
                    (true, format!("create pod {}", name))
                }
            }
            "pod_start" => {
                let name = param_str(task, "pod_name", "");
                (true, format!("start pod {}", name))
            }
            "pod_stop" => {
                let name = param_str(task, "pod_name", "");
                (true, format!("stop pod {}", name))
            }
            "pod_rm" => {
                let name = param_str(task, "pod_name", "");
                let exists = self.pod_exists(exec, name).await?;
                if exists {
                    (true, format!("remove pod {}", name))
                } else {
                    (false, format!("pod {} does not exist", name))
                }
            }
            "generate_systemd" => {
                let name = param_str(task, "container_name", "");
                (true, format!("generate systemd unit for {}", name))
            }
            other => anyhow::bail!("unknown podman action: {}", other),
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
                format!("podman pull {}", esc(image))
            }
            "run" => {
                let image = param_str(task, "image", "");
                let name = param_str(task, "container_name", "");
                let ports = param_str(task, "ports", "");
                let detach = param_bool(task, "detach", true);
                let restart = param_str(task, "restart", "");
                let pod = param_str(task, "pod", "");

                let mut cmd = String::from("podman run");
                if detach {
                    cmd.push_str(" -d");
                }
                if !name.is_empty() {
                    cmd.push_str(&format!(" --name {}", esc(name)));
                }
                if !ports.is_empty() {
                    cmd.push_str(&format!(" -p {}", esc(ports)));
                }
                if !restart.is_empty() {
                    cmd.push_str(&format!(" --restart {}", esc(restart)));
                }
                if !pod.is_empty() {
                    cmd.push_str(&format!(" --pod {}", esc(pod)));
                }
                cmd.push_str(&format!(" {}", esc(image)));
                cmd
            }
            "stop" => {
                let name = param_str(task, "container_name", "");
                format!("podman stop {}", esc(name))
            }
            "rm" => {
                let name = param_str(task, "container_name", "");
                let force = param_bool(task, "force", false);
                if force {
                    format!("podman rm -f {}", esc(name))
                } else {
                    format!("podman rm {}", esc(name))
                }
            }
            "pod_create" => {
                let name = param_str(task, "pod_name", "");
                let ports = param_str(task, "ports", "");
                let mut cmd = format!("podman pod create --name {}", esc(name));
                if !ports.is_empty() {
                    cmd.push_str(&format!(" -p {}", esc(ports)));
                }
                cmd
            }
            "pod_start" => {
                let name = param_str(task, "pod_name", "");
                format!("podman pod start {}", esc(name))
            }
            "pod_stop" => {
                let name = param_str(task, "pod_name", "");
                format!("podman pod stop {}", esc(name))
            }
            "pod_rm" => {
                let name = param_str(task, "pod_name", "");
                let force = param_bool(task, "force", false);
                if force {
                    format!("podman pod rm -f {}", esc(name))
                } else {
                    format!("podman pod rm {}", esc(name))
                }
            }
            "generate_systemd" => {
                let name = param_str(task, "container_name", "");
                let new_flag = param_bool(task, "new", true);
                if new_flag {
                    format!("podman generate systemd --new --name {}", esc(name))
                } else {
                    format!("podman generate systemd --name {}", esc(name))
                }
            }
            other => anyhow::bail!("unknown podman action: {}", other),
        };

        let result = exec.exec(&cmd).await?;

        Ok(TaskResult {
            module: self.name().to_string(),
            action: task.action.clone(),
            success: result.success(),
            changed: result.success(),
            message: if result.success() {
                if task.action == "generate_systemd" {
                    result.stdout // Return the unit file content
                } else {
                    format!("podman {} — ok", task.action)
                }
            } else {
                format!("podman {} — failed: {}", task.action, result.stderr.trim())
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
                let name = param_str(task, "container_name", "");
                if name.is_empty() {
                    return Ok(false);
                }
                self.container_running(exec, name).await
            }
            "stop" => {
                let name = param_str(task, "container_name", "");
                self.container_running(exec, name).await.map(|v| !v)
            }
            "rm" => {
                let name = param_str(task, "container_name", "");
                self.container_exists(exec, name).await.map(|v| !v)
            }
            "pod_create" => {
                let name = param_str(task, "pod_name", "");
                self.pod_exists(exec, name).await
            }
            "pod_rm" => {
                let name = param_str(task, "pod_name", "");
                self.pod_exists(exec, name).await.map(|v| !v)
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_podman_actions() {
        let module = PodmanModule;
        assert_eq!(module.name(), "podman");
        assert!(module.actions().contains(&"pull"));
        assert!(module.actions().contains(&"run"));
        assert!(module.actions().contains(&"pod_create"));
        assert!(module.actions().contains(&"generate_systemd"));
    }
}
