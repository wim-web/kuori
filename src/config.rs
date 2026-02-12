use anyhow::bail;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

// タスクの定義
#[derive(Debug, Deserialize)]
pub struct Task {
    pub name: String,
    pub host: String,
    pub script_path: String,
    pub working_dir: String,
    pub sudo: bool,
    pub environments: HashMap<String, String>,
}

// 設定の定義
#[derive(Debug, Deserialize)]
pub struct Config {
    pub tasks: Vec<Task>,
}

impl Config {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.tasks.is_empty() {
            bail!("`tasks` must contain at least one task");
        }

        let mut seen_names: HashSet<&str> = HashSet::new();

        for (idx, task) in self.tasks.iter().enumerate() {
            validate_non_empty(&task.name, "name", idx)?;
            validate_non_empty(&task.host, "host", idx)?;
            validate_non_empty(&task.script_path, "script_path", idx)?;
            validate_non_empty(&task.working_dir, "working_dir", idx)?;

            if !seen_names.insert(task.name.as_str()) {
                bail!("tasks[{idx}].name is duplicated: {}", task.name);
            }
        }

        Ok(())
    }
}

fn validate_non_empty(value: &str, field_name: &str, task_index: usize) -> anyhow::Result<()> {
    if value.trim().is_empty() {
        bail!("tasks[{task_index}].{field_name} must not be empty");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Config, Task};
    use std::collections::HashMap;

    fn task(name: &str) -> Task {
        Task {
            name: name.to_string(),
            host: "example.com".to_string(),
            script_path: "script.sh".to_string(),
            working_dir: "/tmp".to_string(),
            sudo: false,
            environments: HashMap::new(),
        }
    }

    #[test]
    fn validate_fails_when_tasks_empty() {
        let config = Config { tasks: vec![] };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_fails_when_name_is_blank() {
        let mut blank_name_task = task("deploy");
        blank_name_task.name = "   ".to_string();
        let config = Config {
            tasks: vec![blank_name_task],
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_fails_when_names_duplicated() {
        let config = Config {
            tasks: vec![task("deploy"), task("deploy")],
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_succeeds_with_valid_tasks() {
        let config = Config {
            tasks: vec![task("deploy"), task("cleanup")],
        };
        assert!(config.validate().is_ok());
    }
}
