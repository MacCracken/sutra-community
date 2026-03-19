//! sutra-wireguard — WireGuard VPN interface management.
//!
//! Actions:
//! - `up` — Bring up a WireGuard interface (wg-quick up)
//! - `down` — Bring down a WireGuard interface (wg-quick down)
//! - `sync_config` — Sync config to /etc/wireguard/{interface}.conf

use sutra_core::{Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, esc, param_str};

pub struct WireguardModule;

impl WireguardModule {
    async fn interface_exists(&self, exec: &Executor, iface: &str) -> anyhow::Result<bool> {
        let result = exec
            .exec(&format!("ip link show {} 2>/dev/null", esc(iface)))
            .await?;
        Ok(result.success())
    }
}

impl SutraModule for WireguardModule {
    fn name(&self) -> &str {
        "wireguard"
    }

    fn actions(&self) -> &[&str] {
        &["up", "down", "sync_config"]
    }

    async fn plan(
        &self,
        task: &Task,
        _node: &NodeInfo,
        exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let iface = param_str(task, "interface", "wg0");

        let (changed, description) = match task.action.as_str() {
            "up" => {
                let exists = self.interface_exists(exec, iface).await?;
                if exists {
                    (false, format!("interface '{}' already up", iface))
                } else {
                    (true, format!("bring up interface '{}'", iface))
                }
            }
            "down" => {
                let exists = self.interface_exists(exec, iface).await?;
                if exists {
                    (true, format!("bring down interface '{}'", iface))
                } else {
                    (false, format!("interface '{}' already down", iface))
                }
            }
            "sync_config" => {
                let config = param_str(task, "config", "");
                if config.is_empty() {
                    anyhow::bail!("wireguard.sync_config requires 'config' parameter");
                }
                let conf_path = format!("/etc/wireguard/{}.conf", iface);
                let result = exec
                    .exec(&format!("cat {} 2>/dev/null", esc(&conf_path)))
                    .await?;
                if result.success() && result.stdout.trim() == config.trim() {
                    (false, format!("config for '{}' already in sync", iface))
                } else {
                    (true, format!("sync config for '{}'", iface))
                }
            }
            other => anyhow::bail!("unknown wireguard action: {}", other),
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
        let iface = param_str(task, "interface", "wg0");

        let cmd = match task.action.as_str() {
            "up" => format!("wg-quick up {}", esc(iface)),
            "down" => format!("wg-quick down {}", esc(iface)),
            "sync_config" => {
                let config = param_str(task, "config", "");
                if config.is_empty() {
                    anyhow::bail!("wireguard.sync_config requires 'config' parameter");
                }
                let conf_path = format!("/etc/wireguard/{}.conf", iface);
                format!(
                    "printf %s {} > {} && chmod 600 {}",
                    esc(config),
                    esc(&conf_path),
                    esc(&conf_path)
                )
            }
            other => anyhow::bail!("unknown wireguard action: {}", other),
        };

        let result = exec.exec(&cmd).await?;

        Ok(TaskResult {
            module: self.name().to_string(),
            action: task.action.clone(),
            success: result.success(),
            changed: result.success(),
            message: if result.success() {
                format!("wireguard {} — ok", task.action)
            } else {
                format!(
                    "wireguard {} — failed: {}",
                    task.action,
                    result.stderr.trim()
                )
            },
        })
    }

    async fn check(&self, task: &Task, _node: &NodeInfo, exec: &Executor) -> anyhow::Result<bool> {
        let iface = param_str(task, "interface", "wg0");

        match task.action.as_str() {
            "up" => self.interface_exists(exec, iface).await,
            "down" => self.interface_exists(exec, iface).await.map(|v| !v),
            "sync_config" => {
                let config = param_str(task, "config", "");
                let conf_path = format!("/etc/wireguard/{}.conf", iface);
                let result = exec
                    .exec(&format!("cat {} 2>/dev/null", esc(&conf_path)))
                    .await?;
                Ok(result.success() && result.stdout.trim() == config.trim())
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wireguard_actions() {
        let module = WireguardModule;
        assert_eq!(module.name(), "wireguard");
        assert!(module.actions().contains(&"up"));
        assert!(module.actions().contains(&"down"));
        assert!(module.actions().contains(&"sync_config"));
    }
}
