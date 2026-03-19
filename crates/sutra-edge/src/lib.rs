//! sutra-edge — AGNOS edge node fleet operations.
//!
//! Actions:
//! - `provision` — Provision a new edge node with base AGNOS config
//! - `update` — Trigger OTA update on edge node
//! - `reboot` — Schedule a controlled reboot
//! - `drain` — Drain workloads before maintenance
//! - `label` — Add/remove labels on an edge node

use sutra_core::{param_str, param_bool, Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult};

pub struct EdgeModule;

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
        _exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let description = match task.action.as_str() {
            "provision" => {
                let profile = param_str(task, "profile", "default");
                format!("provision edge node with profile: {}", profile)
            }
            "update" => {
                let channel = param_str(task, "channel", "stable");
                format!("trigger OTA update (channel: {})", channel)
            }
            "reboot" => "schedule controlled reboot".to_string(),
            "drain" => "drain workloads from node".to_string(),
            "label" => {
                let key = param_str(task, "key", "");
                let value = param_str(task, "value", "");
                let remove = param_bool(task, "remove", false);
                if remove {
                    format!("remove label: {}", key)
                } else {
                    format!("set label: {}={}", key, value)
                }
            }
            other => anyhow::bail!("unknown edge action: {}", other),
        };

        Ok(TaskPlan {
            module: self.name().to_string(),
            action: task.action.clone(),
            changed: true,
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
                format!("agnos-edge provision --profile {}", profile)
            }
            "update" => {
                let channel = param_str(task, "channel", "stable");
                format!("agnos-edge update --channel {}", channel)
            }
            "reboot" => "agnos-edge reboot --graceful".to_string(),
            "drain" => "agnos-edge drain".to_string(),
            "label" => {
                let key = param_str(task, "key", "");
                let value = param_str(task, "value", "");
                let remove = param_bool(task, "remove", false);
                if remove {
                    format!("agnos-edge label remove {}", key)
                } else {
                    format!("agnos-edge label set {}={}", key, value)
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

    async fn check(
        &self,
        _task: &Task,
        _node: &NodeInfo,
        _exec: &Executor,
    ) -> anyhow::Result<bool> {
        Ok(false)
    }
}
