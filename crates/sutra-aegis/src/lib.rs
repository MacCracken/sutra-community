//! sutra-aegis — AGNOS security policy enforcement via aegis.
//!
//! Actions:
//! - `apply_policy` — Apply a security policy
//! - `check_compliance` — Verify node meets security requirements
//! - `harden` — Apply hardening profile
//! - `audit` — Run security audit

use sutra_core::{Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, esc, param_str};

pub struct AegisModule;

impl AegisModule {
    async fn active_policy(&self, exec: &Executor) -> anyhow::Result<Option<String>> {
        let result = exec.exec("aegis policy current 2>/dev/null").await?;
        if result.success() && !result.stdout.trim().is_empty() {
            Ok(Some(result.stdout.trim().to_string()))
        } else {
            Ok(None)
        }
    }

    async fn hardening_level(&self, exec: &Executor) -> anyhow::Result<Option<String>> {
        let result = exec.exec("aegis harden --status 2>/dev/null").await?;
        if result.success() && !result.stdout.trim().is_empty() {
            Ok(Some(result.stdout.trim().to_string()))
        } else {
            Ok(None)
        }
    }
}

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
        exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let (changed, description) = match task.action.as_str() {
            "apply_policy" => {
                let policy = param_str(task, "policy", "default");
                let current = self.active_policy(exec).await?;
                if current.as_deref() == Some(policy) {
                    (false, format!("policy '{}' already active", policy))
                } else {
                    (true, format!("apply aegis policy: {}", policy))
                }
            }
            "check_compliance" => {
                let profile = param_str(task, "profile", "standard");
                (
                    false,
                    format!("check compliance against profile: {}", profile),
                )
            }
            "harden" => {
                let level = param_str(task, "level", "standard");
                let current = self.hardening_level(exec).await?;
                if current.as_deref() == Some(level) {
                    (
                        false,
                        format!("hardening level '{}' already applied", level),
                    )
                } else {
                    (true, format!("apply hardening level: {}", level))
                }
            }
            "audit" => (false, "run aegis security audit".to_string()),
            other => anyhow::bail!("unknown aegis action: {}", other),
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
            "apply_policy" => {
                let policy = param_str(task, "policy", "default");
                format!("aegis policy apply {}", esc(policy))
            }
            "check_compliance" => {
                let profile = param_str(task, "profile", "standard");
                format!("aegis compliance check --profile {}", esc(profile))
            }
            "harden" => {
                let level = param_str(task, "level", "standard");
                format!("aegis harden --level {}", esc(level))
            }
            "audit" => "aegis audit".to_string(),
            other => anyhow::bail!("unknown aegis action: {}", other),
        };

        let result = exec.exec(&cmd).await?;

        Ok(TaskResult {
            module: self.name().to_string(),
            action: task.action.clone(),
            success: result.success(),
            changed: result.success()
                && task.action != "check_compliance"
                && task.action != "audit",
            message: if result.success() {
                if result.stdout.trim().is_empty() {
                    format!("aegis {} — ok", task.action)
                } else {
                    result.stdout.trim().to_string()
                }
            } else {
                format!("aegis {} failed: {}", task.action, result.stderr.trim())
            },
        })
    }

    async fn check(&self, task: &Task, _node: &NodeInfo, exec: &Executor) -> anyhow::Result<bool> {
        match task.action.as_str() {
            "apply_policy" => {
                let policy = param_str(task, "policy", "default");
                let current = self.active_policy(exec).await?;
                Ok(current.as_deref() == Some(policy))
            }
            "harden" => {
                let level = param_str(task, "level", "standard");
                let current = self.hardening_level(exec).await?;
                Ok(current.as_deref() == Some(level))
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aegis_actions() {
        let module = AegisModule;
        assert_eq!(module.name(), "aegis");
        assert!(module.actions().contains(&"apply_policy"));
        assert!(module.actions().contains(&"harden"));
        assert!(module.actions().contains(&"audit"));
    }

    #[tokio::test]
    async fn test_aegis_plan_audit() {
        let module = AegisModule;
        let exec = Executor::local();
        let task = Task::new("aegis", "audit", std::collections::HashMap::new());
        let plan = module
            .plan(&task, &NodeInfo::localhost(), &exec)
            .await
            .unwrap();
        assert!(!plan.changed);
    }
}
