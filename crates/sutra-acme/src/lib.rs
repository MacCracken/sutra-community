//! sutra-acme — ACME/Let's Encrypt certificate management.
//!
//! Actions:
//! - `obtain` — Obtain a certificate (certbot certonly)
//! - `renew` — Renew certificates (certbot renew)
//! - `revoke` — Revoke a certificate

use sutra_core::{Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, esc, param_str};

pub struct AcmeModule;

impl AcmeModule {
    async fn cert_valid(&self, exec: &Executor, domain: &str) -> anyhow::Result<bool> {
        let cert_path = format!("/etc/letsencrypt/live/{}/fullchain.pem", domain);
        // Check cert exists and is not expiring within 30 days (2592000 seconds)
        let result = exec
            .exec(&format!(
                "openssl x509 -checkend 2592000 -noout -in {} 2>/dev/null",
                esc(&cert_path)
            ))
            .await?;
        Ok(result.success())
    }

    async fn cert_exists(&self, exec: &Executor, domain: &str) -> anyhow::Result<bool> {
        let cert_path = format!("/etc/letsencrypt/live/{}/fullchain.pem", domain);
        let result = exec.exec(&format!("test -f {}", esc(&cert_path))).await?;
        Ok(result.success())
    }
}

impl SutraModule for AcmeModule {
    fn name(&self) -> &str {
        "acme"
    }

    fn actions(&self) -> &[&str] {
        &["obtain", "renew", "revoke"]
    }

    async fn plan(
        &self,
        task: &Task,
        _node: &NodeInfo,
        exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let domain = param_str(task, "domain", "");

        let (changed, description) = match task.action.as_str() {
            "obtain" => {
                if domain.is_empty() {
                    anyhow::bail!("acme.obtain requires 'domain' parameter");
                }
                let valid = self.cert_valid(exec, domain).await?;
                if valid {
                    (
                        false,
                        format!("certificate for '{}' exists and is valid", domain),
                    )
                } else {
                    (true, format!("obtain certificate for '{}'", domain))
                }
            }
            "renew" => (true, "renew all eligible certificates".to_string()),
            "revoke" => {
                if domain.is_empty() {
                    anyhow::bail!("acme.revoke requires 'domain' parameter");
                }
                let exists = self.cert_exists(exec, domain).await?;
                if exists {
                    (true, format!("revoke certificate for '{}'", domain))
                } else {
                    (false, format!("no certificate found for '{}'", domain))
                }
            }
            other => anyhow::bail!("unknown acme action: {}", other),
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
            "obtain" => {
                let domain = param_str(task, "domain", "");
                let email = param_str(task, "email", "");
                let method = param_str(task, "method", "standalone");
                let webroot = param_str(task, "webroot", "/var/www/html");

                if domain.is_empty() {
                    anyhow::bail!("acme.obtain requires 'domain' parameter");
                }

                let mut cmd = String::from("certbot certonly --non-interactive --agree-tos");

                if !email.is_empty() {
                    cmd.push_str(&format!(" --email {}", esc(email)));
                } else {
                    cmd.push_str(" --register-unsafely-without-email");
                }

                match method {
                    "webroot" => {
                        cmd.push_str(&format!(
                            " --webroot -w {} -d {}",
                            esc(webroot),
                            esc(domain)
                        ));
                    }
                    "dns" => {
                        cmd.push_str(&format!(" --preferred-challenges dns -d {}", esc(domain)));
                    }
                    _ => {
                        // standalone
                        cmd.push_str(&format!(" --standalone -d {}", esc(domain)));
                    }
                }
                cmd
            }
            "renew" => "certbot renew --non-interactive".to_string(),
            "revoke" => {
                let domain = param_str(task, "domain", "");
                if domain.is_empty() {
                    anyhow::bail!("acme.revoke requires 'domain' parameter");
                }
                let cert_path = format!("/etc/letsencrypt/live/{}/fullchain.pem", domain);
                format!(
                    "certbot revoke --non-interactive --cert-path {}",
                    esc(&cert_path)
                )
            }
            other => anyhow::bail!("unknown acme action: {}", other),
        };

        let result = exec.exec(&cmd).await?;

        Ok(TaskResult {
            module: self.name().to_string(),
            action: task.action.clone(),
            success: result.success(),
            changed: result.success(),
            message: if result.success() {
                format!("acme {} — ok", task.action)
            } else {
                format!("acme {} — failed: {}", task.action, result.stderr.trim())
            },
        })
    }

    async fn check(&self, task: &Task, _node: &NodeInfo, exec: &Executor) -> anyhow::Result<bool> {
        match task.action.as_str() {
            "obtain" => {
                let domain = param_str(task, "domain", "");
                self.cert_valid(exec, domain).await
            }
            "revoke" => {
                let domain = param_str(task, "domain", "");
                self.cert_exists(exec, domain).await.map(|v| !v)
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acme_actions() {
        let module = AcmeModule;
        assert_eq!(module.name(), "acme");
        assert!(module.actions().contains(&"obtain"));
        assert!(module.actions().contains(&"renew"));
        assert!(module.actions().contains(&"revoke"));
    }
}
