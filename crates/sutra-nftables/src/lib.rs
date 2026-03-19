//! sutra-nftables — Firewall rule management via nftables.
//!
//! Actions:
//! - `apply_ruleset` — Apply a complete nftables ruleset from a file
//! - `add_rule` — Add a single rule to a chain
//! - `delete_rule` — Remove a rule from a chain
//! - `flush` — Flush all rules (or a specific table/chain)
//! - `list` — List current ruleset

use sutra_core::{param_str, Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult};

pub struct NftablesModule;

impl SutraModule for NftablesModule {
    fn name(&self) -> &str {
        "nftables"
    }

    fn actions(&self) -> &[&str] {
        &["apply_ruleset", "add_rule", "delete_rule", "flush", "list"]
    }

    async fn plan(
        &self,
        task: &Task,
        _node: &NodeInfo,
        _exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let description = match task.action.as_str() {
            "apply_ruleset" => {
                let src = param_str(task, "src", "");
                format!("apply nftables ruleset from {}", src)
            }
            "add_rule" => {
                let table = param_str(task, "table", "filter");
                let chain = param_str(task, "chain", "input");
                let rule = param_str(task, "rule", "");
                format!("add rule to {}/{}: {}", table, chain, rule)
            }
            "delete_rule" => {
                let table = param_str(task, "table", "filter");
                let chain = param_str(task, "chain", "input");
                format!("delete rule from {}/{}", table, chain)
            }
            "flush" => {
                let table = param_str(task, "table", "");
                if table.is_empty() {
                    "flush all nftables rules".to_string()
                } else {
                    format!("flush nftables table {}", table)
                }
            }
            "list" => "list nftables ruleset".to_string(),
            other => anyhow::bail!("unknown nftables action: {}", other),
        };

        Ok(TaskPlan {
            module: self.name().to_string(),
            action: task.action.clone(),
            changed: task.action != "list",
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
            "apply_ruleset" => {
                let src = param_str(task, "src", "");
                if src.is_empty() {
                    anyhow::bail!("nftables.apply_ruleset requires 'src' parameter");
                }
                format!("nft -f {}", src)
            }
            "add_rule" => {
                let table = param_str(task, "table", "filter");
                let chain = param_str(task, "chain", "input");
                let rule = param_str(task, "rule", "");
                format!("nft add rule {} {} {}", table, chain, rule)
            }
            "delete_rule" => {
                let table = param_str(task, "table", "filter");
                let chain = param_str(task, "chain", "input");
                let handle = param_str(task, "handle", "");
                format!("nft delete rule {} {} handle {}", table, chain, handle)
            }
            "flush" => {
                let table = param_str(task, "table", "");
                if table.is_empty() {
                    "nft flush ruleset".to_string()
                } else {
                    format!("nft flush table {}", table)
                }
            }
            "list" => "nft list ruleset".to_string(),
            other => anyhow::bail!("unknown nftables action: {}", other),
        };

        let result = exec.exec(&cmd).await?;

        Ok(TaskResult {
            module: self.name().to_string(),
            action: task.action.clone(),
            success: result.success(),
            changed: result.success() && task.action != "list",
            message: if result.success() {
                if task.action == "list" {
                    result.stdout
                } else {
                    format!("nftables {} — ok", task.action)
                }
            } else {
                format!("nftables {} — failed: {}", task.action, result.stderr.trim())
            },
        })
    }

    async fn check(
        &self,
        _task: &Task,
        _node: &NodeInfo,
        _exec: &Executor,
    ) -> anyhow::Result<bool> {
        // Firewall rules are complex to diff — always apply.
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nftables_actions() {
        let module = NftablesModule;
        assert_eq!(module.name(), "nftables");
        assert!(module.actions().contains(&"apply_ruleset"));
        assert!(module.actions().contains(&"add_rule"));
        assert!(module.actions().contains(&"flush"));
    }
}
