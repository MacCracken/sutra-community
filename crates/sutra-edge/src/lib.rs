//! sutra-edge — AGNOS edge node fleet operations.
//!
//! Actions:
//! - `provision` — Provision a new edge node with base AGNOS config
//! - `update` — Trigger OTA update on edge node
//! - `reboot` — Schedule a controlled reboot
//! - `drain` — Drain workloads before maintenance
//! - `label` — Add/remove labels on an edge node

use sutra_core::{
    Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, esc, param_bool, param_str,
};

pub struct EdgeModule;

impl EdgeModule {
    async fn is_provisioned(&self, exec: &Executor) -> anyhow::Result<bool> {
        exec.file_exists("/etc/agnos/edge.conf").await
    }

    async fn has_label(&self, exec: &Executor, key: &str) -> anyhow::Result<bool> {
        let result = exec
            .exec(&format!("agnos-edge label get {} 2>/dev/null", esc(key)))
            .await?;
        Ok(result.success() && !result.stdout.trim().is_empty())
    }
}

impl SutraModule for EdgeModule {
    fn name(&self) -> &str {
        "edge"
    }

    fn actions(&self) -> &[&str] {
        &["provision", "update", "reboot", "drain", "label"]
    }

    async fn plan(
        &self,
        task: &Task,
        _node: &NodeInfo,
        exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let (changed, description) = match task.action.as_str() {
            "provision" => {
                let profile = param_str(task, "profile", "default");
                let provisioned = self.is_provisioned(exec).await?;
                if provisioned {
                    (
                        false,
                        format!("node already provisioned (profile: {})", profile),
                    )
                } else {
                    (
                        true,
                        format!("provision edge node with profile: {}", profile),
                    )
                }
            }
            "update" => {
                let channel = param_str(task, "channel", "stable");
                (true, format!("trigger OTA update (channel: {})", channel))
            }
            "reboot" => (true, "schedule controlled reboot".to_string()),
            "drain" => (true, "drain workloads from node".to_string()),
            "label" => {
                let key = param_str(task, "key", "");
                let value = param_str(task, "value", "");
                let remove = param_bool(task, "remove", false);
                if remove {
                    let has = self.has_label(exec, key).await?;
                    if has {
                        (true, format!("remove label: {}", key))
                    } else {
                        (false, format!("label {} already absent", key))
                    }
                } else {
                    (true, format!("set label: {}={}", key, value))
                }
            }
            other => anyhow::bail!("unknown edge action: {}", other),
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
            "provision" => {
                let profile = param_str(task, "profile", "default");
                format!("agnos-edge provision --profile {}", esc(profile))
            }
            "update" => {
                let channel = param_str(task, "channel", "stable");
                format!("agnos-edge update --channel {}", esc(channel))
            }
            "reboot" => "agnos-edge reboot --graceful".to_string(),
            "drain" => "agnos-edge drain".to_string(),
            "label" => {
                let key = param_str(task, "key", "");
                let value = param_str(task, "value", "");
                let remove = param_bool(task, "remove", false);
                if remove {
                    format!("agnos-edge label remove {}", esc(key))
                } else {
                    format!("agnos-edge label set {}={}", esc(key), esc(value))
                }
            }
            other => anyhow::bail!("unknown edge action: {}", other),
        };

        let result = exec.exec(&cmd).await?;

        Ok(TaskResult {
            module: self.name().to_string(),
            action: task.action.clone(),
            success: result.success(),
            changed: result.success(),
            message: if result.success() {
                format!("edge {} — ok", task.action)
            } else {
                format!("edge {} — failed: {}", task.action, result.stderr.trim())
            },
        })
    }

    async fn check(&self, task: &Task, _node: &NodeInfo, exec: &Executor) -> anyhow::Result<bool> {
        match task.action.as_str() {
            "provision" => self.is_provisioned(exec).await,
            "label" => {
                let key = param_str(task, "key", "");
                let remove = param_bool(task, "remove", false);
                if remove {
                    self.has_label(exec, key).await.map(|v| !v)
                } else {
                    self.has_label(exec, key).await
                }
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_actions() {
        let module = EdgeModule;
        assert_eq!(module.name(), "edge");
        assert!(module.actions().contains(&"provision"));
        assert!(module.actions().contains(&"update"));
        assert!(module.actions().contains(&"label"));
    }

    #[tokio::test]
    async fn test_edge_plan_reboot() {
        let module = EdgeModule;
        let exec = Executor::local();
        let task = Task::new("edge", "reboot", std::collections::HashMap::new());
        let plan = module
            .plan(&task, &NodeInfo::localhost(), &exec)
            .await
            .unwrap();
        assert!(plan.changed);
    }
}
