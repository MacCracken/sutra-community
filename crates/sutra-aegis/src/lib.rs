//! sutra-aegis — AGNOS security policy enforcement via aegis.
//!
//! Actions:
//! - `apply_policy` — Apply a security policy
//! - `check_compliance` — Verify node meets security requirements
//! - `harden` — Apply hardening profile
//! - `audit` — Run security audit

use sutra_core::{param_str, Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult};

pub struct AegisModule;

impl SutraModule for AegisModule {
    fn name(&self) -> &str {
        "aegis"
    }

    fn actions(&self) -> &[&str] {
        &["apply_policy", "check_compliance", "harden", "audit"]
    }

    async fn plan(
        &self,
        task: &Task,
        _node: &NodeInfo,
        _exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let description = match task.action.as_str() {
            "apply_policy" => {
                let policy = param_str(task, "policy", "default");
                format!("apply aegis policy: {}", policy)
            }
            "check_compliance" => {
                let profile = param_str(task, "profile", "standard");
                format!("check compliance against profile: {}", profile)
            }
            "harden" => {
                let level = param_str(task, "level", "standard");
                format!("apply hardening level: {}", level)
            }
            "audit" => "run aegis security audit".to_string(),
            other => anyhow::bail!("unknown aegis action: {}", other),
        };

        Ok(TaskPlan {
            module: self.name().to_string(),
            action: task.action.clone(),
            changed: task.action != "check_compliance" && task.action != "audit",
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
            "apply_policy" => {
                let policy = param_str(task, "policy", "default");
                format!("aegis policy apply {}", policy)
            }
            "check_compliance" => {
                let profile = param_str(task, "profile", "standard");
                format!("aegis compliance check --profile {}", profile)
            }
            "harden" => {
                let level = param_str(task, "level", "standard");
                format!("aegis harden --level {}", level)
            }
            "audit" => "aegis audit".to_string(),
            other => anyhow::bail!("unknown aegis action: {}", other),
        };

        let result = exec.exec(&cmd).await?;

        Ok(TaskResult {
            module: self.name().to_string(),
            action: task.action.clone(),
            success: result.success(),
            changed: result.success() && task.action != "check_compliance" && task.action != "audit",
            message: if result.success() {
                result.stdout.trim().to_string()
            } else {
                format!("aegis {} failed: {}", task.action, result.stderr.trim())
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
