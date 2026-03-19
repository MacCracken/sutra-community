//! sutra-daimon — AGNOS daimon agent lifecycle and fleet reporting.
//!
//! Actions:
//! - `report` — Report status/event to daimon fleet controller
//! - `register` — Register this node with the fleet
//! - `deregister` — Remove this node from the fleet
//! - `heartbeat` — Send heartbeat to fleet controller

use sutra_core::{Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, esc, param_str};

pub struct DaimonModule;

impl DaimonModule {
    fn endpoint(task: &Task) -> &str {
        param_str(task, "endpoint", "http://localhost:8090")
    }

    async fn is_registered(&self, exec: &Executor, task: &Task) -> anyhow::Result<bool> {
        let endpoint = Self::endpoint(task);
        let result = exec
            .exec(&format!(
                "curl -sf {}/v1/agents/self 2>/dev/null",
                esc(endpoint)
            ))
            .await?;
        Ok(result.success())
    }
}

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
        exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let (changed, description) = match task.action.as_str() {
            "report" => {
                let status = param_str(task, "status", "");
                (true, format!("report to daimon: {}", status))
            }
            "register" => {
                let registered = self.is_registered(exec, task).await.unwrap_or(false);
                if registered {
                    (false, "node already registered with daimon".to_string())
                } else {
                    (true, "register node with daimon fleet".to_string())
                }
            }
            "deregister" => {
                let registered = self.is_registered(exec, task).await.unwrap_or(false);
                if registered {
                    (true, "deregister node from daimon fleet".to_string())
                } else {
                    (false, "node not registered with daimon".to_string())
                }
            }
            "heartbeat" => (true, "send heartbeat to daimon".to_string()),
            other => anyhow::bail!("unknown daimon action: {}", other),
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
        let endpoint = esc(Self::endpoint(task));
        let cmd = match task.action.as_str() {
            "report" => {
                let status = param_str(task, "status", "");
                format!(
                    "curl -sf -X POST {}/v1/reports -H 'Content-Type: application/json' -d '{{\"status\":{}}}'",
                    endpoint,
                    esc(&format!("\"{}\"", status.replace('"', "\\\"")))
                )
            }
            "register" => {
                format!("curl -sf -X POST {}/v1/agents/register", endpoint)
            }
            "deregister" => {
                format!("curl -sf -X DELETE {}/v1/agents/self", endpoint)
            }
            "heartbeat" => {
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

    async fn check(&self, task: &Task, _node: &NodeInfo, exec: &Executor) -> anyhow::Result<bool> {
        match task.action.as_str() {
            "register" => self.is_registered(exec, task).await,
            "deregister" => self.is_registered(exec, task).await.map(|v| !v),
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daimon_actions() {
        let module = DaimonModule;
        assert_eq!(module.name(), "daimon");
        assert!(module.actions().contains(&"register"));
        assert!(module.actions().contains(&"report"));
        assert!(module.actions().contains(&"heartbeat"));
    }
}
