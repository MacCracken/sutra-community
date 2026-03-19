//! sutra-redis — Redis instance management.
//!
//! Actions:
//! - `set` — Set a key-value pair (redis-cli SET)
//! - `get` — Get a key value
//! - `del` — Delete a key
//! - `flush` — Flush a database (FLUSHDB)
//! - `ping` — Check redis is responding

use sutra_core::{
    Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, esc, param_int, param_str,
};

pub struct RedisModule;

impl RedisModule {
    fn cli_prefix(task: &Task) -> String {
        let host = param_str(task, "host", "localhost");
        let port = param_int(task, "port", 6379);
        let db = param_int(task, "db", 0);
        let password = param_str(task, "password", "");

        let mut cmd = format!("redis-cli -h {} -p {} -n {}", esc(host), port, db);
        if !password.is_empty() {
            cmd.push_str(&format!(" -a {}", esc(password)));
        }
        cmd
    }

    async fn key_value(
        &self,
        exec: &Executor,
        task: &Task,
        key: &str,
    ) -> anyhow::Result<Option<String>> {
        let prefix = Self::cli_prefix(task);
        let result = exec.exec(&format!("{} GET {}", prefix, esc(key))).await?;
        if result.success() {
            let val = result.stdout.trim().to_string();
            if val == "(nil)" {
                Ok(None)
            } else {
                Ok(Some(val))
            }
        } else {
            Ok(None)
        }
    }

    async fn key_exists(&self, exec: &Executor, task: &Task, key: &str) -> anyhow::Result<bool> {
        let prefix = Self::cli_prefix(task);
        let result = exec
            .exec(&format!("{} EXISTS {}", prefix, esc(key)))
            .await?;
        Ok(result.success() && result.stdout.trim() == "1")
    }
}

impl SutraModule for RedisModule {
    fn name(&self) -> &str {
        "redis"
    }

    fn actions(&self) -> &[&str] {
        &["set", "get", "del", "flush", "ping"]
    }

    async fn plan(
        &self,
        task: &Task,
        _node: &NodeInfo,
        exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let (changed, description) = match task.action.as_str() {
            "set" => {
                let key = param_str(task, "key", "");
                let value = param_str(task, "value", "");
                if key.is_empty() {
                    anyhow::bail!("redis.set requires 'key' parameter");
                }
                let current = self.key_value(exec, task, key).await?;
                if current.as_deref() == Some(value) {
                    (false, format!("key '{}' already has correct value", key))
                } else {
                    (true, format!("set key '{}'", key))
                }
            }
            "get" => {
                let key = param_str(task, "key", "");
                (true, format!("get key '{}'", key))
            }
            "del" => {
                let key = param_str(task, "key", "");
                if key.is_empty() {
                    anyhow::bail!("redis.del requires 'key' parameter");
                }
                let exists = self.key_exists(exec, task, key).await?;
                if exists {
                    (true, format!("delete key '{}'", key))
                } else {
                    (false, format!("key '{}' already absent", key))
                }
            }
            "flush" => (true, "flush database".to_string()),
            "ping" => (false, "ping redis".to_string()),
            other => anyhow::bail!("unknown redis action: {}", other),
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
        let prefix = Self::cli_prefix(task);

        let cmd = match task.action.as_str() {
            "set" => {
                let key = param_str(task, "key", "");
                let value = param_str(task, "value", "");
                if key.is_empty() {
                    anyhow::bail!("redis.set requires 'key' parameter");
                }
                format!("{} SET {} {}", prefix, esc(key), esc(value))
            }
            "get" => {
                let key = param_str(task, "key", "");
                format!("{} GET {}", prefix, esc(key))
            }
            "del" => {
                let key = param_str(task, "key", "");
                if key.is_empty() {
                    anyhow::bail!("redis.del requires 'key' parameter");
                }
                format!("{} DEL {}", prefix, esc(key))
            }
            "flush" => format!("{} FLUSHDB", prefix),
            "ping" => format!("{} PING", prefix),
            other => anyhow::bail!("unknown redis action: {}", other),
        };

        let result = exec.exec(&cmd).await?;

        Ok(TaskResult {
            module: self.name().to_string(),
            action: task.action.clone(),
            success: result.success(),
            changed: result.success() && task.action != "get" && task.action != "ping",
            message: if result.success() {
                format!("redis {} — {}", task.action, result.stdout.trim())
            } else {
                format!("redis {} — failed: {}", task.action, result.stderr.trim())
            },
        })
    }

    async fn check(&self, task: &Task, _node: &NodeInfo, exec: &Executor) -> anyhow::Result<bool> {
        match task.action.as_str() {
            "set" => {
                let key = param_str(task, "key", "");
                let value = param_str(task, "value", "");
                let current = self.key_value(exec, task, key).await?;
                Ok(current.as_deref() == Some(value))
            }
            "del" => {
                let key = param_str(task, "key", "");
                self.key_exists(exec, task, key).await.map(|v| !v)
            }
            "ping" => {
                let prefix = Self::cli_prefix(task);
                let result = exec.exec(&format!("{} PING", prefix)).await?;
                Ok(result.success() && result.stdout.trim() == "PONG")
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_actions() {
        let module = RedisModule;
        assert_eq!(module.name(), "redis");
        assert!(module.actions().contains(&"set"));
        assert!(module.actions().contains(&"get"));
        assert!(module.actions().contains(&"del"));
        assert!(module.actions().contains(&"flush"));
        assert!(module.actions().contains(&"ping"));
    }
}
