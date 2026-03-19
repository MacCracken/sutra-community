//! sutra-k8s — Kubernetes resource management via kubectl.
//!
//! Actions:
//! - `apply` — Apply a manifest (kubectl apply -f)
//! - `delete` — Delete a resource
//! - `rollout` — Manage rollouts (status, restart, undo)
//! - `scale` — Scale a deployment/statefulset
//! - `wait` — Wait for a condition on a resource
//! - `exec` — Execute a command in a pod

use sutra_core::{
    Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, esc, param_int, param_str,
};

pub struct K8sModule;

impl K8sModule {
    fn kubectl_base(task: &Task) -> String {
        let context = param_str(task, "context", "");
        let namespace = param_str(task, "namespace", "");
        let kubeconfig = param_str(task, "kubeconfig", "");

        let mut cmd = String::from("kubectl");
        if !context.is_empty() {
            cmd.push_str(&format!(" --context {}", esc(context)));
        }
        if !namespace.is_empty() {
            cmd.push_str(&format!(" -n {}", esc(namespace)));
        }
        if !kubeconfig.is_empty() {
            cmd.push_str(&format!(" --kubeconfig {}", esc(kubeconfig)));
        }
        cmd
    }

    async fn resource_exists(
        &self,
        exec: &Executor,
        task: &Task,
        kind: &str,
        name: &str,
    ) -> anyhow::Result<bool> {
        let base = Self::kubectl_base(task);
        let result = exec
            .exec(&format!(
                "{} get {} {} -o name 2>/dev/null",
                base,
                esc(kind),
                esc(name)
            ))
            .await?;
        Ok(result.success())
    }
}

impl SutraModule for K8sModule {
    fn name(&self) -> &str {
        "k8s"
    }

    fn actions(&self) -> &[&str] {
        &["apply", "delete", "rollout", "scale", "wait", "exec"]
    }

    async fn plan(
        &self,
        task: &Task,
        _node: &NodeInfo,
        exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let (changed, description) = match task.action.as_str() {
            "apply" => {
                let manifest = param_str(task, "manifest", "");
                (true, format!("apply manifest {}", manifest))
            }
            "delete" => {
                let kind = param_str(task, "kind", "");
                let resource_name = param_str(task, "resource_name", "");
                let exists = self
                    .resource_exists(exec, task, kind, resource_name)
                    .await
                    .unwrap_or(false);
                if exists {
                    (true, format!("delete {} {}", kind, resource_name))
                } else {
                    (false, format!("{} {} does not exist", kind, resource_name))
                }
            }
            "rollout" => {
                let sub = param_str(task, "sub_action", "status");
                let kind = param_str(task, "kind", "deployment");
                let resource_name = param_str(task, "resource_name", "");
                let is_mutation = sub != "status";
                (
                    is_mutation,
                    format!("rollout {} {}/{}", sub, kind, resource_name),
                )
            }
            "scale" => {
                let kind = param_str(task, "kind", "deployment");
                let resource_name = param_str(task, "resource_name", "");
                let replicas = param_int(task, "replicas", 1);
                (
                    true,
                    format!("scale {}/{} to {} replicas", kind, resource_name, replicas),
                )
            }
            "wait" => {
                let kind = param_str(task, "kind", "");
                let resource_name = param_str(task, "resource_name", "");
                let condition = param_str(task, "condition", "Ready");
                (
                    false,
                    format!(
                        "wait for {} {}/{} to be {}",
                        condition, kind, resource_name, condition
                    ),
                )
            }
            "exec" => {
                let pod = param_str(task, "pod", "");
                let cmd = param_str(task, "cmd", "");
                (true, format!("exec in pod {}: {}", pod, cmd))
            }
            other => anyhow::bail!("unknown k8s action: {}", other),
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
        let base = Self::kubectl_base(task);
        let cmd = match task.action.as_str() {
            "apply" => {
                let manifest = param_str(task, "manifest", "");
                if manifest.is_empty() {
                    anyhow::bail!("k8s.apply requires 'manifest' parameter");
                }
                format!("{} apply -f {}", base, esc(manifest))
            }
            "delete" => {
                let kind = param_str(task, "kind", "");
                let resource_name = param_str(task, "resource_name", "");
                format!(
                    "{} delete {} {} --ignore-not-found",
                    base,
                    esc(kind),
                    esc(resource_name)
                )
            }
            "rollout" => {
                let sub = param_str(task, "sub_action", "status");
                let kind = param_str(task, "kind", "deployment");
                let resource_name = param_str(task, "resource_name", "");
                format!(
                    "{} rollout {} {}/{}",
                    base,
                    esc(sub),
                    esc(kind),
                    esc(resource_name)
                )
            }
            "scale" => {
                let kind = param_str(task, "kind", "deployment");
                let resource_name = param_str(task, "resource_name", "");
                let replicas = param_int(task, "replicas", 1);
                format!(
                    "{} scale {}/{} --replicas={}",
                    base,
                    esc(kind),
                    esc(resource_name),
                    replicas
                )
            }
            "wait" => {
                let kind = param_str(task, "kind", "");
                let resource_name = param_str(task, "resource_name", "");
                let condition = param_str(task, "condition", "Ready");
                let timeout = param_int(task, "timeout", 300);
                format!(
                    "{} wait {}/{} --for=condition={} --timeout={}s",
                    base,
                    esc(kind),
                    esc(resource_name),
                    esc(condition),
                    timeout
                )
            }
            "exec" => {
                let pod = param_str(task, "pod", "");
                let container = param_str(task, "container", "");
                let cmd_str = param_str(task, "cmd", "");
                let mut kubectl = format!("{} exec {}", base, esc(pod));
                if !container.is_empty() {
                    kubectl.push_str(&format!(" -c {}", esc(container)));
                }
                kubectl.push_str(&format!(" -- {}", cmd_str)); // cmd intentionally unescaped (user's command)
                kubectl
            }
            other => anyhow::bail!("unknown k8s action: {}", other),
        };

        let result = exec.exec(&cmd).await?;

        Ok(TaskResult {
            module: self.name().to_string(),
            action: task.action.clone(),
            success: result.success(),
            changed: result.success()
                && task.action != "wait"
                && !(task.action == "rollout"
                    && param_str(task, "sub_action", "status") == "status"),
            message: if result.success() {
                let out = result.stdout.trim();
                if out.is_empty() {
                    format!("k8s {} — ok", task.action)
                } else {
                    out.to_string()
                }
            } else {
                format!("k8s {} — failed: {}", task.action, result.stderr.trim())
            },
        })
    }

    async fn check(&self, task: &Task, _node: &NodeInfo, exec: &Executor) -> anyhow::Result<bool> {
        match task.action.as_str() {
            "delete" => {
                let kind = param_str(task, "kind", "");
                let resource_name = param_str(task, "resource_name", "");
                self.resource_exists(exec, task, kind, resource_name)
                    .await
                    .map(|v| !v)
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_k8s_actions() {
        let module = K8sModule;
        assert_eq!(module.name(), "k8s");
        assert!(module.actions().contains(&"apply"));
        assert!(module.actions().contains(&"scale"));
        assert!(module.actions().contains(&"rollout"));
        assert!(module.actions().contains(&"wait"));
    }
}
