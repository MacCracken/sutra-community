//! sutra-hoosh — LLM gateway management via hoosh.
//!
//! Manage local LLM inference, model lifecycle, provider config, and token budgets.
//!
//! Actions:
//! - `pull_model` — Pull a model from registry (HuggingFace, Ollama library)
//! - `list_models` — List locally stored models
//! - `remove_model` — Remove a model from local store
//! - `set_provider` — Configure a cloud LLM provider (API key, endpoint)
//! - `token_budget` — Set per-agent or global token budget
//! - `restart` — Restart the hoosh service
//! - `status` — Show hoosh health, loaded models, GPU usage

use sutra_core::{Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, param_str};

pub struct HooshModule;

impl SutraModule for HooshModule {
    fn name(&self) -> &str {
        "hoosh"
    }

    fn actions(&self) -> &[&str] {
        &["pull_model", "list_models", "remove_model", "set_provider", "token_budget", "restart", "status"]
    }

    async fn plan(
        &self,
        task: &Task,
        _node: &NodeInfo,
        _exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let description = match task.action.as_str() {
            "pull_model" => {
                let model = param_str(task, "model")?;
                format!("Pull model: {model}")
            }
            "list_models" => "List locally stored models".into(),
            "remove_model" => {
                let model = param_str(task, "model")?;
                format!("Remove model: {model}")
            }
            "set_provider" => {
                let provider = param_str(task, "provider")?;
                format!("Configure LLM provider: {provider}")
            }
            "token_budget" => {
                let pool = param_str(task, "pool").unwrap_or_else(|_| "global".into());
                let limit = param_str(task, "limit")?;
                format!("Set token budget for '{pool}': {limit}")
            }
            "restart" => "Restart hoosh service".into(),
            "status" => "Show hoosh status".into(),
            other => anyhow::bail!("unknown hoosh action: {other}"),
        };

        Ok(TaskPlan {
            description,
            changes: vec![],
            needs_privilege: task.action == "restart" || task.action == "set_provider",
        })
    }

    async fn apply(
        &self,
        task: &Task,
        _node: &NodeInfo,
        exec: &Executor,
    ) -> anyhow::Result<TaskResult> {
        let hoosh_api = param_str(task, "api_url")
            .unwrap_or_else(|_| "http://localhost:8088".into());

        match task.action.as_str() {
            "pull_model" => {
                let model = param_str(task, "model")?;
                exec.run_command(&format!(
                    "curl -sf -X POST {hoosh_api}/v1/models/pull -H 'Content-Type: application/json' -d '{{\"model\":\"{model}\"}}'"
                )).await
            }
            "list_models" => {
                exec.run_command(&format!(
                    "curl -sf {hoosh_api}/v1/models/local"
                )).await
            }
            "remove_model" => {
                let model = param_str(task, "model")?;
                exec.run_command(&format!(
                    "curl -sf -X DELETE {hoosh_api}/v1/models/{model}"
                )).await
            }
            "set_provider" => {
                let provider = param_str(task, "provider")?;
                let api_key = param_str(task, "api_key")?;
                let endpoint = param_str(task, "endpoint").unwrap_or_else(|_| String::new());
                let config = format!(
                    "[providers.{provider}]\napi_key = \"{api_key}\"\n{}\n",
                    if endpoint.is_empty() { String::new() } else { format!("endpoint = \"{endpoint}\"") }
                );
                exec.write_file(&format!("/etc/agnos/hoosh/providers/{provider}.toml"), &config).await
            }
            "token_budget" => {
                let pool = param_str(task, "pool").unwrap_or_else(|_| "global".into());
                let limit = param_str(task, "limit")?;
                exec.run_command(&format!(
                    "curl -sf -X POST {hoosh_api}/v1/tokens/pools -H 'Content-Type: application/json' -d '{{\"name\":\"{pool}\",\"limit\":{limit}}}'"
                )).await
            }
            "restart" => {
                exec.run_command("systemctl restart hoosh").await
            }
            "status" => {
                exec.run_command(&format!("curl -sf {hoosh_api}/v1/health")).await
            }
            other => anyhow::bail!("unknown hoosh action: {other}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_name() {
        assert_eq!(HooshModule.name(), "hoosh");
    }

    #[test]
    fn module_actions() {
        let actions = HooshModule.actions();
        assert!(actions.contains(&"pull_model"));
        assert!(actions.contains(&"list_models"));
        assert!(actions.contains(&"token_budget"));
        assert!(actions.contains(&"status"));
        assert_eq!(actions.len(), 7);
    }
}
