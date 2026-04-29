//! sutra-ark — Package management via ark.
//!
//! Install, update, remove, and verify AGNOS packages across fleet nodes.
//!
//! Actions:
//! - `install` — Install a package (from ark repo or marketplace)
//! - `update` — Update a package to latest version
//! - `remove` — Remove an installed package
//! - `sync` — Sync package repository metadata
//! - `verify` — Verify installed package integrity (signatures + checksums)
//! - `list` — List installed packages
//! - `pin` — Pin a package to a specific version (prevent auto-update)

use sutra_core::{Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, param_str};

pub struct ArkModule;

impl SutraModule for ArkModule {
    fn name(&self) -> &str {
        "ark"
    }

    fn actions(&self) -> &[&str] {
        &["install", "update", "remove", "sync", "verify", "list", "pin"]
    }

    async fn plan(
        &self,
        task: &Task,
        _node: &NodeInfo,
        _exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let description = match task.action.as_str() {
            "install" => {
                let package = param_str(task, "package")?;
                let version = param_str(task, "version").unwrap_or_else(|_| "latest".into());
                format!("Install package: {package} ({version})")
            }
            "update" => {
                let package = param_str(task, "package")
                    .unwrap_or_else(|_| "all".into());
                format!("Update package: {package}")
            }
            "remove" => {
                let package = param_str(task, "package")?;
                format!("Remove package: {package}")
            }
            "sync" => "Sync package repository metadata".into(),
            "verify" => {
                let package = param_str(task, "package")
                    .unwrap_or_else(|_| "all".into());
                format!("Verify package integrity: {package}")
            }
            "list" => "List installed packages".into(),
            "pin" => {
                let package = param_str(task, "package")?;
                let version = param_str(task, "version")?;
                format!("Pin {package} to version {version}")
            }
            other => anyhow::bail!("unknown ark action: {other}"),
        };

        Ok(TaskPlan {
            description,
            changes: vec![],
            needs_privilege: matches!(task.action.as_str(), "install" | "update" | "remove" | "sync"),
        })
    }

    async fn apply(
        &self,
        task: &Task,
        _node: &NodeInfo,
        exec: &Executor,
    ) -> anyhow::Result<TaskResult> {
        match task.action.as_str() {
            "install" => {
                let package = param_str(task, "package")?;
                let version = param_str(task, "version").unwrap_or_else(|_| "latest".into());
                if version == "latest" {
                    exec.run_command(&format!("ark install {package}")).await
                } else {
                    exec.run_command(&format!("ark install {package}={version}")).await
                }
            }
            "update" => {
                let package = param_str(task, "package")
                    .unwrap_or_else(|_| "all".into());
                if package == "all" {
                    exec.run_command("ark update").await
                } else {
                    exec.run_command(&format!("ark update {package}")).await
                }
            }
            "remove" => {
                let package = param_str(task, "package")?;
                exec.run_command(&format!("ark remove {package}")).await
            }
            "sync" => {
                exec.run_command("ark sync").await
            }
            "verify" => {
                let package = param_str(task, "package")
                    .unwrap_or_else(|_| "all".into());
                if package == "all" {
                    exec.run_command("ark verify --all").await
                } else {
                    exec.run_command(&format!("ark verify {package}")).await
                }
            }
            "list" => {
                exec.run_command("ark list --format json").await
            }
            "pin" => {
                let package = param_str(task, "package")?;
                let version = param_str(task, "version")?;
                exec.run_command(&format!("ark pin {package}={version}")).await
            }
            other => anyhow::bail!("unknown ark action: {other}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_name() {
        assert_eq!(ArkModule.name(), "ark");
    }

    #[test]
    fn module_actions() {
        let actions = ArkModule.actions();
        assert!(actions.contains(&"install"));
        assert!(actions.contains(&"update"));
        assert!(actions.contains(&"remove"));
        assert!(actions.contains(&"verify"));
        assert!(actions.contains(&"pin"));
        assert_eq!(actions.len(), 7);
    }
}
