//! sutra-nginx — Nginx reverse proxy management.
//!
//! Actions:
//! - `site_enable` — Enable a site config (symlink sites-available to sites-enabled, reload nginx)
//! - `site_disable` — Disable a site (remove symlink from sites-enabled, reload nginx)
//! - `config_test` — Test nginx configuration (nginx -t)
//! - `reload` — Reload nginx
//! - `restart` — Restart nginx

use sutra_core::{Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, esc, param_str};

pub struct NginxModule;

impl NginxModule {
    async fn site_enabled(&self, exec: &Executor, site: &str) -> anyhow::Result<bool> {
        let result = exec
            .exec(&format!("test -L /etc/nginx/sites-enabled/{}", esc(site)))
            .await?;
        Ok(result.success())
    }
}

impl SutraModule for NginxModule {
    fn name(&self) -> &str {
        "nginx"
    }

    fn actions(&self) -> &[&str] {
        &[
            "site_enable",
            "site_disable",
            "config_test",
            "reload",
            "restart",
        ]
    }

    async fn plan(
        &self,
        task: &Task,
        _node: &NodeInfo,
        exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let (changed, description) = match task.action.as_str() {
            "site_enable" => {
                let site = param_str(task, "site", "");
                if site.is_empty() {
                    anyhow::bail!("nginx.site_enable requires 'site' parameter");
                }
                let enabled = self.site_enabled(exec, site).await?;
                if enabled {
                    (false, format!("site '{}' already enabled", site))
                } else {
                    (true, format!("enable site '{}'", site))
                }
            }
            "site_disable" => {
                let site = param_str(task, "site", "");
                if site.is_empty() {
                    anyhow::bail!("nginx.site_disable requires 'site' parameter");
                }
                let enabled = self.site_enabled(exec, site).await?;
                if enabled {
                    (true, format!("disable site '{}'", site))
                } else {
                    (false, format!("site '{}' already disabled", site))
                }
            }
            "config_test" => (true, "test nginx configuration".to_string()),
            "reload" => (true, "reload nginx".to_string()),
            "restart" => (true, "restart nginx".to_string()),
            other => anyhow::bail!("unknown nginx action: {}", other),
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
        let cmd = match task.action.as_str() {
            "site_enable" => {
                let site = param_str(task, "site", "");
                if site.is_empty() {
                    anyhow::bail!("nginx.site_enable requires 'site' parameter");
                }
                format!(
                    "ln -sf /etc/nginx/sites-available/{site} /etc/nginx/sites-enabled/{site} && nginx -s reload",
                    site = esc(site)
                )
            }
            "site_disable" => {
                let site = param_str(task, "site", "");
                if site.is_empty() {
                    anyhow::bail!("nginx.site_disable requires 'site' parameter");
                }
                format!(
                    "rm -f /etc/nginx/sites-enabled/{site} && nginx -s reload",
                    site = esc(site)
                )
            }
            "config_test" => "nginx -t".to_string(),
            "reload" => "nginx -s reload".to_string(),
            "restart" => "systemctl restart nginx".to_string(),
            other => anyhow::bail!("unknown nginx action: {}", other),
        };

        let result = exec.exec(&cmd).await?;

        Ok(TaskResult {
            module: self.name().to_string(),
            action: task.action.clone(),
            success: result.success(),
            changed: result.success(),
            message: if result.success() {
                format!("nginx {} — ok", task.action)
            } else {
                format!("nginx {} — failed: {}", task.action, result.stderr.trim())
            },
        })
    }

    async fn check(&self, task: &Task, _node: &NodeInfo, exec: &Executor) -> anyhow::Result<bool> {
        match task.action.as_str() {
            "site_enable" => {
                let site = param_str(task, "site", "");
                self.site_enabled(exec, site).await
            }
            "site_disable" => {
                let site = param_str(task, "site", "");
                self.site_enabled(exec, site).await.map(|v| !v)
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nginx_actions() {
        let module = NginxModule;
        assert_eq!(module.name(), "nginx");
        assert!(module.actions().contains(&"site_enable"));
        assert!(module.actions().contains(&"site_disable"));
        assert!(module.actions().contains(&"config_test"));
        assert!(module.actions().contains(&"reload"));
        assert!(module.actions().contains(&"restart"));
    }
}
