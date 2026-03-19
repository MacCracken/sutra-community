//! sutra-postgres — PostgreSQL database management.
//!
//! Actions:
//! - `create_db` — Create a database
//! - `drop_db` — Drop a database
//! - `create_user` — Create a database user/role
//! - `drop_user` — Drop a database user/role
//! - `grant` — Grant privileges
//! - `query` — Execute a SQL query (read-only assertion)

use sutra_core::{
    Executor, NodeInfo, SutraModule, Task, TaskPlan, TaskResult, esc, param_bool, param_str,
};

pub struct PostgresModule;

impl PostgresModule {
    fn psql_cmd(task: &Task, sql: &str) -> String {
        let host = param_str(task, "host", "localhost");
        let port = param_str(task, "port", "5432");
        let pg_user = param_str(task, "pg_user", "postgres");
        format!(
            "psql -h {} -p {} -U {} -tAc {}",
            esc(host),
            esc(port),
            esc(pg_user),
            esc(sql)
        )
    }

    async fn db_exists(&self, exec: &Executor, task: &Task, db: &str) -> anyhow::Result<bool> {
        let sql = format!(
            "SELECT 1 FROM pg_database WHERE datname='{}'",
            db.replace('\'', "''")
        );
        let result = exec.exec(&Self::psql_cmd(task, &sql)).await?;
        Ok(result.success() && result.stdout.trim() == "1")
    }

    async fn user_exists(&self, exec: &Executor, task: &Task, user: &str) -> anyhow::Result<bool> {
        let sql = format!(
            "SELECT 1 FROM pg_roles WHERE rolname='{}'",
            user.replace('\'', "''")
        );
        let result = exec.exec(&Self::psql_cmd(task, &sql)).await?;
        Ok(result.success() && result.stdout.trim() == "1")
    }
}

impl SutraModule for PostgresModule {
    fn name(&self) -> &str {
        "postgres"
    }

    fn actions(&self) -> &[&str] {
        &[
            "create_db",
            "drop_db",
            "create_user",
            "drop_user",
            "grant",
            "query",
        ]
    }

    async fn plan(
        &self,
        task: &Task,
        _node: &NodeInfo,
        exec: &Executor,
    ) -> anyhow::Result<TaskPlan> {
        let (changed, description) = match task.action.as_str() {
            "create_db" => {
                let db = param_str(task, "database", "");
                let exists = self.db_exists(exec, task, db).await.unwrap_or(false);
                if exists {
                    (false, format!("database {} already exists", db))
                } else {
                    (true, format!("create database {}", db))
                }
            }
            "drop_db" => {
                let db = param_str(task, "database", "");
                let exists = self.db_exists(exec, task, db).await.unwrap_or(false);
                if exists {
                    (true, format!("drop database {}", db))
                } else {
                    (false, format!("database {} does not exist", db))
                }
            }
            "create_user" => {
                let user = param_str(task, "username", "");
                let exists = self.user_exists(exec, task, user).await.unwrap_or(false);
                if exists {
                    (false, format!("user {} already exists", user))
                } else {
                    (true, format!("create user {}", user))
                }
            }
            "drop_user" => {
                let user = param_str(task, "username", "");
                let exists = self.user_exists(exec, task, user).await.unwrap_or(false);
                if exists {
                    (true, format!("drop user {}", user))
                } else {
                    (false, format!("user {} does not exist", user))
                }
            }
            "grant" => {
                let privs = param_str(task, "privileges", "ALL");
                let db = param_str(task, "database", "");
                let user = param_str(task, "username", "");
                (true, format!("grant {} on {} to {}", privs, db, user))
            }
            "query" => {
                let sql = param_str(task, "sql", "");
                (false, format!("query: {}", &sql[..sql.len().min(60)]))
            }
            other => anyhow::bail!("unknown postgres action: {}", other),
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
        let sql = match task.action.as_str() {
            "create_db" => {
                let db = param_str(task, "database", "");
                let owner = param_str(task, "owner", "");
                if owner.is_empty() {
                    format!("CREATE DATABASE \"{}\"", db.replace('"', "\"\""))
                } else {
                    format!(
                        "CREATE DATABASE \"{}\" OWNER \"{}\"",
                        db.replace('"', "\"\""),
                        owner.replace('"', "\"\"")
                    )
                }
            }
            "drop_db" => {
                let db = param_str(task, "database", "");
                format!("DROP DATABASE IF EXISTS \"{}\"", db.replace('"', "\"\""))
            }
            "create_user" => {
                let user = param_str(task, "username", "");
                let password = param_str(task, "password", "");
                let superuser = param_bool(task, "superuser", false);
                let mut sql = format!("CREATE ROLE \"{}\"", user.replace('"', "\"\""));
                sql.push_str(" LOGIN");
                if !password.is_empty() {
                    sql.push_str(&format!(" PASSWORD '{}'", password.replace('\'', "''")));
                }
                if superuser {
                    sql.push_str(" SUPERUSER");
                }
                sql
            }
            "drop_user" => {
                let user = param_str(task, "username", "");
                format!("DROP ROLE IF EXISTS \"{}\"", user.replace('"', "\"\""))
            }
            "grant" => {
                let privs = param_str(task, "privileges", "ALL");
                let db = param_str(task, "database", "");
                let user = param_str(task, "username", "");
                format!(
                    "GRANT {} ON DATABASE \"{}\" TO \"{}\"",
                    privs,
                    db.replace('"', "\"\""),
                    user.replace('"', "\"\"")
                )
            }
            "query" => param_str(task, "sql", "").to_string(),
            other => anyhow::bail!("unknown postgres action: {}", other),
        };

        let result = exec.exec(&Self::psql_cmd(task, &sql)).await?;

        Ok(TaskResult {
            module: self.name().to_string(),
            action: task.action.clone(),
            success: result.success(),
            changed: result.success() && task.action != "query",
            message: if result.success() {
                if task.action == "query" {
                    result.stdout.trim().to_string()
                } else {
                    format!("postgres {} — ok", task.action)
                }
            } else {
                format!(
                    "postgres {} — failed: {}",
                    task.action,
                    result.stderr.trim()
                )
            },
        })
    }

    async fn check(&self, task: &Task, _node: &NodeInfo, exec: &Executor) -> anyhow::Result<bool> {
        match task.action.as_str() {
            "create_db" => {
                let db = param_str(task, "database", "");
                self.db_exists(exec, task, db).await
            }
            "drop_db" => {
                let db = param_str(task, "database", "");
                self.db_exists(exec, task, db).await.map(|v| !v)
            }
            "create_user" => {
                let user = param_str(task, "username", "");
                self.user_exists(exec, task, user).await
            }
            "drop_user" => {
                let user = param_str(task, "username", "");
                self.user_exists(exec, task, user).await.map(|v| !v)
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postgres_actions() {
        let module = PostgresModule;
        assert_eq!(module.name(), "postgres");
        assert!(module.actions().contains(&"create_db"));
        assert!(module.actions().contains(&"grant"));
        assert!(module.actions().contains(&"query"));
    }
}
