//! sutra-cron — Cron job management.
//!
//! Actions:
//! - `present` — Ensure a cron job exists
//! - `absent` — Ensure a cron job does not exist

use sutra_core::{Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, esc, param_str};

pub struct CronModule;

impl CronModule {
    fn job_id(task: &Task) -> String {
        let name = param_str(task, "name", "");
        format!("# sutra:{}", name)
    }

    async fn job_exists(&self, exec: &Executor, task: &Task) -> anyhow::Result<bool> {
        let user = param_str(task, "user", "root");
        let marker = Self::job_id(task);
        let result = exec
            .exec(&format!(
                "crontab -l -u {} 2>/dev/null | grep -qF {}",
                esc(user),
                esc(&marker)
            ))
            .await?;
        Ok(result.success())
    }
}

impl SutraModule for CronModule {
    fn name(&self) -> &str {
        "cron"
    }

    fn actions(&self) -> &[&str] {
        &["present", "absent"]
    }

    async fn plan(
        &self,
        task: &Task,
        _node: &NodeInfo,
        exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let job_name = param_str(task, "name", "");
        let (changed, description) = match task.action.as_str() {
            "present" => {
                let schedule = param_str(task, "schedule", "");
                let exists = self.job_exists(exec, task).await?;
                if exists {
                    (false, format!("cron job '{}' already present", job_name))
                } else {
                    (true, format!("add cron job '{}' ({})", job_name, schedule))
                }
            }
            "absent" => {
                let exists = self.job_exists(exec, task).await?;
                if exists {
                    (true, format!("remove cron job '{}'", job_name))
                } else {
                    (false, format!("cron job '{}' already absent", job_name))
                }
            }
            other => anyhow::bail!("unknown cron action: {}", other),
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
        let user = param_str(task, "user", "root");
        let job_name = param_str(task, "name", "");
        let marker = Self::job_id(task);

        match task.action.as_str() {
            "present" => {
                let schedule = param_str(task, "schedule", "");
                let cmd = param_str(task, "cmd", "");
                if schedule.is_empty() || cmd.is_empty() {
                    anyhow::bail!("cron.present requires 'schedule' and 'cmd' parameters");
                }

                // Remove old entry if exists, then append new one.
                let cron_line = format!("{} {} {}", schedule, cmd, marker);
                let script = format!(
                    "(crontab -l -u {user} 2>/dev/null | grep -vF {marker}; echo {line}) | crontab -u {user} -",
                    user = esc(user),
                    marker = esc(&marker),
                    line = esc(&cron_line),
                );
                let result = exec.exec(&script).await?;

                Ok(TaskResult {
                    module: self.name().to_string(),
                    action: task.action.clone(),
                    success: result.success(),
                    changed: result.success(),
                    message: if result.success() {
                        format!("cron job '{}' added", job_name)
                    } else {
                        format!("failed to add cron job: {}", result.stderr.trim())
                    },
                })
            }
            "absent" => {
                let script = format!(
                    "crontab -l -u {user} 2>/dev/null | grep -vF {marker} | crontab -u {user} -",
                    user = esc(user),
                    marker = esc(&marker),
                );
                let result = exec.exec(&script).await?;

                Ok(TaskResult {
                    module: self.name().to_string(),
                    action: task.action.clone(),
                    success: result.success(),
                    changed: result.success(),
                    message: if result.success() {
                        format!("cron job '{}' removed", job_name)
                    } else {
                        format!("failed to remove cron job: {}", result.stderr.trim())
                    },
                })
            }
            other => anyhow::bail!("unknown cron action: {}", other),
        }
    }

    async fn check(&self, task: &Task, _node: &NodeInfo, exec: &Executor) -> anyhow::Result<bool> {
        match task.action.as_str() {
            "present" => self.job_exists(exec, task).await,
            "absent" => self.job_exists(exec, task).await.map(|v| !v),
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_actions() {
        let module = CronModule;
        assert_eq!(module.name(), "cron");
        assert!(module.actions().contains(&"present"));
        assert!(module.actions().contains(&"absent"));
    }
}
