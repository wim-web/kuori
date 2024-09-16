use serde::Deserialize;
use std::collections::HashMap;

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
