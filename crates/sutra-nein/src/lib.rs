//! sutra-nein — Firewall policy management via nein.
//!
//! Higher-level firewall management using nein's policy engine instead of raw nftables.
//! Manages container networking, service mesh policy, and agent access control.
//!
//! Actions:
//! - `apply_policy` — Apply a nein firewall policy (TOML file or inline)
//! - `allow_agent` — Grant an agent network access (by agent ID, port, protocol)
//! - `deny_agent` — Revoke an agent's network access
//! - `list_policies` — List active nein policies
//! - `container_network` — Configure container networking rules (bridge, isolation)
//! - `status` — Show firewall status summary (active rules, blocked count)

use sutra_core::{Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, param_str};

pub struct NeinModule;

impl SutraModule for NeinModule {
    fn name(&self) -> &str {
        "nein"
    }

    fn actions(&self) -> &[&str] {
        &["apply_policy", "allow_agent", "deny_agent", "list_policies", "container_network", "status"]
    }

    async fn plan(
        &self,
        task: &Task,
        _node: &NodeInfo,
        _exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let description = match task.action.as_str() {
            "apply_policy" => {
                let policy = param_str(task, "policy")?;
                format!("Apply nein firewall policy: {policy}")
            }
            "allow_agent" => {
                let agent_id = param_str(task, "agent_id")?;
                let port = param_str(task, "port").unwrap_or_else(|_| "any".into());
                format!("Allow agent {agent_id} network access on port {port}")
            }
            "deny_agent" => {
                let agent_id = param_str(task, "agent_id")?;
                format!("Deny agent {agent_id} network access")
            }
            "list_policies" => "List active nein firewall policies".into(),
            "container_network" => {
                let network = param_str(task, "network")?;
                let mode = param_str(task, "mode").unwrap_or_else(|_| "bridge".into());
                format!("Configure container network '{network}' in {mode} mode")
            }
            "status" => "Show nein firewall status".into(),
            other => anyhow::bail!("unknown nein action: {other}"),
        };

        Ok(TaskPlan {
            description,
            changes: vec![],
            needs_privilege: task.action != "list_policies" && task.action != "status",
        })
    }

    async fn apply(
        &self,
        task: &Task,
        _node: &NodeInfo,
        exec: &Executor,
    ) -> anyhow::Result<TaskResult> {
        match task.action.as_str() {
            "apply_policy" => {
                let policy = param_str(task, "policy")?;
                exec.run_command(&format!("nein apply {policy}")).await
            }
            "allow_agent" => {
                let agent_id = param_str(task, "agent_id")?;
                let port = param_str(task, "port").unwrap_or_else(|_| "any".into());
                let protocol = param_str(task, "protocol").unwrap_or_else(|_| "tcp".into());
                exec.run_command(&format!(
                    "nein allow --agent {agent_id} --port {port} --protocol {protocol}"
                )).await
            }
            "deny_agent" => {
                let agent_id = param_str(task, "agent_id")?;
                exec.run_command(&format!("nein deny --agent {agent_id}")).await
            }
            "list_policies" => {
                exec.run_command("nein list --format json").await
            }
            "container_network" => {
                let network = param_str(task, "network")?;
                let mode = param_str(task, "mode").unwrap_or_else(|_| "bridge".into());
                let subnet = param_str(task, "subnet").unwrap_or_else(|_| "172.20.0.0/16".into());
                exec.run_command(&format!(
                    "nein container-network --name {network} --mode {mode} --subnet {subnet}"
                )).await
            }
            "status" => {
                exec.run_command("nein status --format json").await
            }
            other => anyhow::bail!("unknown nein action: {other}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_name() {
        assert_eq!(NeinModule.name(), "nein");
    }

    #[test]
    fn module_actions() {
        let actions = NeinModule.actions();
        assert!(actions.contains(&"apply_policy"));
        assert!(actions.contains(&"allow_agent"));
        assert!(actions.contains(&"deny_agent"));
        assert!(actions.contains(&"container_network"));
        assert!(actions.contains(&"status"));
        assert_eq!(actions.len(), 6);
    }
}
