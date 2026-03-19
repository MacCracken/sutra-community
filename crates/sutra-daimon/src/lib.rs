//! sutra-daimon — AGNOS daimon agent lifecycle and fleet reporting.
//!
//! Actions:
//! - `report` — Report status/event to daimon fleet controller
//! - `register` — Register this node with the fleet
//! - `deregister` — Remove this node from the fleet
//! - `heartbeat` — Send heartbeat to fleet controller

use sutra_core::{param_str, Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult};

pub struct DaimonModule;

impl SutraModule for DaimonModule {
    fn name(&self) -> &str {
        "daimon"
    }

    fn actions(&self) -> &[&str] {
        &["report", "register", "deregister", "heartbeat"]
    }

    async fn plan(
        &self,
        task: &Task,
        _node: &NodeInfo,
        _exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let description = match task.action.as_str() {
            "report" => {
                let status = param_str(task, "status", "");
                format!("report to daimon: {}", status)
            }
            "register" => "register node with daimon fleet".to_string(),
            "deregister" => "deregister node from daimon fleet".to_string(),
            "heartbeat" => "send heartbeat to daimon".to_string(),
            other => anyhow::bail!("unknown daimon action: {}", other),
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
        // daimon operations go through the daimon HTTP API or CLI.
        let cmd = match task.action.as_str() {
            "report" => {
                let status = param_str(task, "status", "");
                let endpoint = param_str(task, "endpoint", "http://localhost:8090");
                format!(
                    "curl -sf -X POST {}/v1/reports -H 'Content-Type: application/json' -d '{{\"status\":\"{}\"}}'",
                    endpoint, status
                )
            }
            "register" => {
                let endpoint = param_str(task, "endpoint", "http://localhost:8090");
                format!("curl -sf -X POST {}/v1/agents/register", endpoint)
            }
            "deregister" => {
                let endpoint = param_str(task, "endpoint", "http://localhost:8090");
                format!("curl -sf -X DELETE {}/v1/agents/self", endpoint)
            }
            "heartbeat" => {
                let endpoint = param_str(task, "endpoint", "http://localhost:8090");
                format!("curl -sf -X POST {}/v1/agents/heartbeat", endpoint)
            }
            other => anyhow::bail!("unknown daimon action: {}", other),
        };

        let result = exec.exec(&cmd).await?;

        Ok(TaskResult {
            module: self.name().to_string(),
            action: task.action.clone(),
            success: result.success(),
            changed: result.success(),
            message: if result.success() {
                format!("daimon {} — ok", task.action)
            } else {
                format!("daimon {} — failed: {}", task.action, result.stderr.trim())
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
