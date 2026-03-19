//! sutra-sysctl — Kernel parameter management via sysctl.
//!
//! Actions:
//! - `set` — Set a sysctl parameter (persistent via /etc/sysctl.d/)
//! - `get` — Read a sysctl parameter value
//! - `apply` — Apply all parameters from a sysctl.conf file

use sutra_core::{Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, param_str};

pub struct SysctlModule;

impl SysctlModule {
    async fn current_value(&self, exec: &Executor, key: &str) -> anyhow::Result<Option<String>> {
        let result = exec.exec(&format!("sysctl -n {} 2>/dev/null", key)).await?;
        if result.success() {
            Ok(Some(result.stdout.trim().to_string()))
        } else {
            Ok(None)
        }
    }
}

impl SutraModule for SysctlModule {
    fn name(&self) -> &str {
        "sysctl"
    }

    fn actions(&self) -> &[&str] {
        &["set", "get", "apply"]
    }

    async fn plan(
        &self,
        task: &Task,
        _node: &NodeInfo,
        exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let (changed, description) = match task.action.as_str() {
            "set" => {
                let key = param_str(task, "key", "");
                let value = param_str(task, "value", "");
                let current = self.current_value(exec, key).await?;
                match current {
                    Some(v) if v == value => (false, format!("{} = {} (already set)", key, value)),
                    Some(v) => (true, format!("{}: {} -> {}", key, v, value)),
                    None => (true, format!("set {} = {}", key, value)),
                }
            }
            "get" => {
                let key = param_str(task, "key", "");
                (false, format!("read sysctl {}", key))
            }
            "apply" => {
                let src = param_str(task, "src", "");
                (true, format!("apply sysctl config from {}", src))
            }
            other => anyhow::bail!("unknown sysctl action: {}", other),
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
        match task.action.as_str() {
            "set" => {
                let key = param_str(task, "key", "");
                let value = param_str(task, "value", "");
                let persist = param_str(task, "persist", "true") == "true";

                // Set runtime value.
                let result = exec.exec(&format!("sysctl -w {}={}", key, value)).await?;

                if !result.success() {
                    return Ok(TaskResult {
                        module: self.name().to_string(),
                        action: task.action.clone(),
                        success: false,
                        changed: false,
                        message: format!("sysctl set failed: {}", result.stderr.trim()),
                    });
                }

                // Persist to /etc/sysctl.d/ if requested.
                if persist {
                    let filename = key.replace('.', "-");
                    let conf_line = format!("{}={}\n", key, value);
                    exec.write_file(
                        &format!("/etc/sysctl.d/99-sutra-{}.conf", filename),
                        conf_line.as_bytes(),
                        0o644,
                    )
                    .await?;
                }

                Ok(TaskResult {
                    module: self.name().to_string(),
                    action: task.action.clone(),
                    success: true,
                    changed: true,
                    message: format!("set {} = {}", key, value),
                })
            }
            "get" => {
                let key = param_str(task, "key", "");
                let result = exec.exec(&format!("sysctl -n {}", key)).await?;
                Ok(TaskResult {
                    module: self.name().to_string(),
                    action: task.action.clone(),
                    success: result.success(),
                    changed: false,
                    message: result.stdout.trim().to_string(),
                })
            }
            "apply" => {
                let src = param_str(task, "src", "");
                let result = exec.exec(&format!("sysctl -p {}", src)).await?;
                Ok(TaskResult {
                    module: self.name().to_string(),
                    action: task.action.clone(),
                    success: result.success(),
                    changed: result.success(),
                    message: if result.success() {
                        format!("applied sysctl config from {}", src)
                    } else {
                        format!("failed: {}", result.stderr.trim())
                    },
                })
            }
            other => anyhow::bail!("unknown sysctl action: {}", other),
        }
    }

    async fn check(&self, task: &Task, _node: &NodeInfo, exec: &Executor) -> anyhow::Result<bool> {
        match task.action.as_str() {
            "set" => {
                let key = param_str(task, "key", "");
                let value = param_str(task, "value", "");
                let current = self.current_value(exec, key).await?;
                Ok(current.as_deref() == Some(value))
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sysctl_actions() {
        let module = SysctlModule;
        assert_eq!(module.name(), "sysctl");
        assert!(module.actions().contains(&"set"));
        assert!(module.actions().contains(&"get"));
    }
}
