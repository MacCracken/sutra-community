//! sutra-mount — Filesystem mount and fstab management.
//!
//! Actions:
//! - `mounted` — Ensure a filesystem is mounted
//! - `unmounted` — Ensure a filesystem is unmounted
//! - `fstab` — Ensure an fstab entry exists

use sutra_core::{Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, esc, param_str};

pub struct MountModule;

impl MountModule {
    async fn is_mounted(&self, exec: &Executor, mountpoint: &str) -> anyhow::Result<bool> {
        let result = exec
            .exec(&format!("mountpoint -q {}", esc(mountpoint)))
            .await?;
        Ok(result.success())
    }

    async fn fstab_has_entry(&self, exec: &Executor, mountpoint: &str) -> anyhow::Result<bool> {
        let result = exec
            .exec(&format!("grep -qsE '\\s{}\\s' /etc/fstab", esc(mountpoint)))
            .await?;
        Ok(result.success())
    }
}

impl SutraModule for MountModule {
    fn name(&self) -> &str {
        "mount"
    }

    fn actions(&self) -> &[&str] {
        &["mounted", "unmounted", "fstab"]
    }

    async fn plan(
        &self,
        task: &Task,
        _node: &NodeInfo,
        exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let mountpoint = param_str(task, "path", "");

        let (changed, description) = match task.action.as_str() {
            "mounted" => {
                let src = param_str(task, "src", "");
                let mounted = self.is_mounted(exec, mountpoint).await?;
                if mounted {
                    (false, format!("{} already mounted", mountpoint))
                } else {
                    (true, format!("mount {} on {}", src, mountpoint))
                }
            }
            "unmounted" => {
                let mounted = self.is_mounted(exec, mountpoint).await?;
                if mounted {
                    (true, format!("unmount {}", mountpoint))
                } else {
                    (false, format!("{} already unmounted", mountpoint))
                }
            }
            "fstab" => {
                let has = self.fstab_has_entry(exec, mountpoint).await?;
                if has {
                    (
                        false,
                        format!("fstab entry for {} already present", mountpoint),
                    )
                } else {
                    (true, format!("add fstab entry for {}", mountpoint))
                }
            }
            other => anyhow::bail!("unknown mount action: {}", other),
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
        let mountpoint = param_str(task, "path", "");

        match task.action.as_str() {
            "mounted" => {
                let src = param_str(task, "src", "");
                let fstype = param_str(task, "fstype", "auto");
                let opts = param_str(task, "opts", "defaults");

                // Create mountpoint if needed, then mount.
                let cmd = format!(
                    "mkdir -p {} && mount -t {} -o {} {} {}",
                    esc(mountpoint),
                    esc(fstype),
                    esc(opts),
                    esc(src),
                    esc(mountpoint)
                );
                let result = exec.exec(&cmd).await?;

                Ok(TaskResult {
                    module: self.name().to_string(),
                    action: task.action.clone(),
                    success: result.success(),
                    changed: result.success(),
                    message: if result.success() {
                        format!("mounted {} on {}", src, mountpoint)
                    } else {
                        format!("mount failed: {}", result.stderr.trim())
                    },
                })
            }
            "unmounted" => {
                let result = exec.exec(&format!("umount {}", esc(mountpoint))).await?;

                Ok(TaskResult {
                    module: self.name().to_string(),
                    action: task.action.clone(),
                    success: result.success(),
                    changed: result.success(),
                    message: if result.success() {
                        format!("unmounted {}", mountpoint)
                    } else {
                        format!("umount failed: {}", result.stderr.trim())
                    },
                })
            }
            "fstab" => {
                let src = param_str(task, "src", "");
                let fstype = param_str(task, "fstype", "auto");
                let opts = param_str(task, "opts", "defaults");
                let dump = param_str(task, "dump", "0");
                let pass = param_str(task, "pass", "0");

                let line = format!(
                    "{} {} {} {} {} {}",
                    src, mountpoint, fstype, opts, dump, pass
                );
                let cmd = format!("echo {} >> /etc/fstab", esc(&line));
                let result = exec.exec(&cmd).await?;

                Ok(TaskResult {
                    module: self.name().to_string(),
                    action: task.action.clone(),
                    success: result.success(),
                    changed: result.success(),
                    message: if result.success() {
                        format!("added fstab entry for {}", mountpoint)
                    } else {
                        format!("fstab update failed: {}", result.stderr.trim())
                    },
                })
            }
            other => anyhow::bail!("unknown mount action: {}", other),
        }
    }

    async fn check(&self, task: &Task, _node: &NodeInfo, exec: &Executor) -> anyhow::Result<bool> {
        let mountpoint = param_str(task, "path", "");
        match task.action.as_str() {
            "mounted" => self.is_mounted(exec, mountpoint).await,
            "unmounted" => self.is_mounted(exec, mountpoint).await.map(|v| !v),
            "fstab" => self.fstab_has_entry(exec, mountpoint).await,
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mount_actions() {
        let module = MountModule;
        assert_eq!(module.name(), "mount");
        assert!(module.actions().contains(&"mounted"));
        assert!(module.actions().contains(&"fstab"));
    }
}
