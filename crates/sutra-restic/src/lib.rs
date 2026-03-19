//! sutra-restic — Restic backup orchestration.
//!
//! Actions:
//! - `init` — Initialize a backup repository
//! - `backup` — Run a backup of specified paths
//! - `forget` — Prune old snapshots with retention policy
//! - `check` — Verify repository integrity
//! - `snapshots` — List snapshots (read-only)

use sutra_core::{
    Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, esc, param_int, param_str,
};

pub struct ResticModule;

impl ResticModule {
    fn env_prefix(task: &Task) -> String {
        let repo = param_str(task, "repo", "");
        let password_file = param_str(task, "password_file", "");

        let mut prefix = String::new();
        if !repo.is_empty() {
            prefix.push_str(&format!("RESTIC_REPOSITORY={} ", esc(repo)));
        }
        if !password_file.is_empty() {
            prefix.push_str(&format!("RESTIC_PASSWORD_FILE={} ", esc(password_file)));
        }
        prefix
    }

    async fn repo_initialized(&self, exec: &Executor, task: &Task) -> anyhow::Result<bool> {
        let env = Self::env_prefix(task);
        let result = exec
            .exec(&format!("{}restic snapshots --no-lock 2>/dev/null", env))
            .await?;
        Ok(result.success())
    }
}

impl SutraModule for ResticModule {
    fn name(&self) -> &str {
        "restic"
    }

    fn actions(&self) -> &[&str] {
        &["init", "backup", "forget", "check", "snapshots"]
    }

    async fn plan(
        &self,
        task: &Task,
        _node: &NodeInfo,
        exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let (changed, description) = match task.action.as_str() {
            "init" => {
                let repo = param_str(task, "repo", "");
                let initialized = self.repo_initialized(exec, task).await?;
                if initialized {
                    (false, format!("repository '{}' already initialized", repo))
                } else {
                    (true, format!("initialize repository '{}'", repo))
                }
            }
            "backup" => {
                let paths = param_str(task, "paths", "");
                (true, format!("backup paths: {}", paths))
            }
            "forget" => {
                let keep_daily = param_int(task, "keep_daily", 7);
                let keep_weekly = param_int(task, "keep_weekly", 4);
                let keep_monthly = param_int(task, "keep_monthly", 12);
                (
                    true,
                    format!(
                        "prune snapshots (daily:{}, weekly:{}, monthly:{})",
                        keep_daily, keep_weekly, keep_monthly
                    ),
                )
            }
            "check" => (true, "verify repository integrity".to_string()),
            "snapshots" => (false, "list snapshots".to_string()),
            other => anyhow::bail!("unknown restic action: {}", other),
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
        let env = Self::env_prefix(task);

        let cmd = match task.action.as_str() {
            "init" => format!("{}restic init", env),
            "backup" => {
                let paths = param_str(task, "paths", "");
                if paths.is_empty() {
                    anyhow::bail!("restic.backup requires 'paths' parameter");
                }
                let mut cmd = format!("{}restic backup", env);
                for path in paths.split_whitespace() {
                    cmd.push_str(&format!(" {}", esc(path)));
                }
                cmd
            }
            "forget" => {
                let keep_daily = param_int(task, "keep_daily", 7);
                let keep_weekly = param_int(task, "keep_weekly", 4);
                let keep_monthly = param_int(task, "keep_monthly", 12);
                format!(
                    "{}restic forget --prune --keep-daily {} --keep-weekly {} --keep-monthly {}",
                    env, keep_daily, keep_weekly, keep_monthly
                )
            }
            "check" => format!("{}restic check", env),
            "snapshots" => format!("{}restic snapshots", env),
            other => anyhow::bail!("unknown restic action: {}", other),
        };

        let result = exec.exec(&cmd).await?;

        Ok(TaskResult {
            module: self.name().to_string(),
            action: task.action.clone(),
            success: result.success(),
            changed: result.success() && task.action != "snapshots",
            message: if result.success() {
                format!("restic {} — ok", task.action)
            } else {
                format!("restic {} — failed: {}", task.action, result.stderr.trim())
            },
        })
    }

    async fn check(&self, task: &Task, _node: &NodeInfo, exec: &Executor) -> anyhow::Result<bool> {
        match task.action.as_str() {
            "init" => self.repo_initialized(exec, task).await,
            "check" => {
                let env = Self::env_prefix(task);
                let result = exec.exec(&format!("{}restic check", env)).await?;
                Ok(result.success())
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restic_actions() {
        let module = ResticModule;
        assert_eq!(module.name(), "restic");
        assert!(module.actions().contains(&"init"));
        assert!(module.actions().contains(&"backup"));
        assert!(module.actions().contains(&"forget"));
        assert!(module.actions().contains(&"check"));
        assert!(module.actions().contains(&"snapshots"));
    }
}
