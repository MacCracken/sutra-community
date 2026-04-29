//! sutra-yukti — Device management via yukti.
//!
//! Hardware provisioning and device lifecycle management.
//! Manages USB devices, block devices, optical drives, udev rules.
//!
//! Actions:
//! - `list_devices` — List all detected devices (USB, block, optical)
//! - `mount` — Mount a block device or USB drive
//! - `unmount` — Safely unmount and eject a device
//! - `udev_rule` — Install a udev rule for device auto-configuration
//! - `format` — Format a block device (ext4, btrfs, xfs)
//! - `hotplug_policy` — Configure hotplug behavior (auto-mount, permissions, agent access)

use sutra_core::{Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, param_str};

pub struct YuktiModule;

impl SutraModule for YuktiModule {
    fn name(&self) -> &str {
        "yukti"
    }

    fn actions(&self) -> &[&str] {
        &["list_devices", "mount", "unmount", "udev_rule", "format", "hotplug_policy"]
    }

    async fn plan(
        &self,
        task: &Task,
        _node: &NodeInfo,
        _exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let description = match task.action.as_str() {
            "list_devices" => "List all detected hardware devices".into(),
            "mount" => {
                let device = param_str(task, "device")?;
                let mountpoint = param_str(task, "mountpoint")?;
                format!("Mount {device} at {mountpoint}")
            }
            "unmount" => {
                let mountpoint = param_str(task, "mountpoint")?;
                format!("Unmount and eject {mountpoint}")
            }
            "udev_rule" => {
                let rule_name = param_str(task, "name")?;
                format!("Install udev rule: {rule_name}")
            }
            "format" => {
                let device = param_str(task, "device")?;
                let filesystem = param_str(task, "filesystem").unwrap_or_else(|_| "ext4".into());
                format!("Format {device} as {filesystem}")
            }
            "hotplug_policy" => {
                let policy = param_str(task, "policy").unwrap_or_else(|_| "manual".into());
                format!("Set hotplug policy: {policy}")
            }
            other => anyhow::bail!("unknown yukti action: {other}"),
        };

        Ok(TaskPlan {
            description,
            changes: vec![],
            needs_privilege: task.action != "list_devices",
        })
    }

    async fn apply(
        &self,
        task: &Task,
        _node: &NodeInfo,
        exec: &Executor,
    ) -> anyhow::Result<TaskResult> {
        match task.action.as_str() {
            "list_devices" => {
                exec.run_command("yukti list --format json").await
            }
            "mount" => {
                let device = param_str(task, "device")?;
                let mountpoint = param_str(task, "mountpoint")?;
                let filesystem = param_str(task, "filesystem").unwrap_or_else(|_| "auto".into());
                let options = param_str(task, "options").unwrap_or_else(|_| "defaults".into());
                exec.run_command(&format!(
                    "mount -t {filesystem} -o {options} {device} {mountpoint}"
                )).await
            }
            "unmount" => {
                let mountpoint = param_str(task, "mountpoint")?;
                exec.run_command(&format!("umount {mountpoint}")).await
            }
            "udev_rule" => {
                let name = param_str(task, "name")?;
                let content = param_str(task, "content")?;
                let rule_path = format!("/etc/udev/rules.d/99-{name}.rules");
                exec.write_file(&rule_path, &content).await?;
                exec.run_command("udevadm control --reload-rules && udevadm trigger").await
            }
            "format" => {
                let device = param_str(task, "device")?;
                let filesystem = param_str(task, "filesystem").unwrap_or_else(|_| "ext4".into());
                let label = param_str(task, "label").unwrap_or_else(|_| String::new());
                let label_flag = if label.is_empty() {
                    String::new()
                } else {
                    format!(" -L {label}")
                };
                exec.run_command(&format!("mkfs.{filesystem}{label_flag} {device}")).await
            }
            "hotplug_policy" => {
                let policy = param_str(task, "policy").unwrap_or_else(|_| "manual".into());
                let config = format!(
                    "[hotplug]\npolicy = \"{policy}\"\nauto_mount = {}\n",
                    policy == "auto"
                );
                exec.write_file("/etc/agnos/yukti/hotplug.toml", &config).await?;
                exec.run_command("systemctl restart yukti-hotplug").await
            }
            other => anyhow::bail!("unknown yukti action: {other}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_name() {
        assert_eq!(YuktiModule.name(), "yukti");
    }

    #[test]
    fn module_actions() {
        let actions = YuktiModule.actions();
        assert!(actions.contains(&"list_devices"));
        assert!(actions.contains(&"mount"));
        assert!(actions.contains(&"unmount"));
        assert!(actions.contains(&"format"));
        assert_eq!(actions.len(), 6);
    }
}
