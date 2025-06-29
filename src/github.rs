use crate::config::load_config;
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PublishError {
    #[error("Ошибка ввода/вывода: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Ошибка выполнения BunJS: {0}")]
    ExecutionError(String),
    #[error("Ошибка загрузки конфигурации: {0}")]
    ConfigError(#[from] Box<dyn std::error::Error>),
}

pub fn publish_html() -> Result<(), PublishError> {
    let config = load_config()?;
    std::env::set_var("GITHUB_TOKEN", config.github.token);

    let output = Command::new("bun")
        .arg("run")
        .arg("publish.js")
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(PublishError::ExecutionError(error));
    }

    println!("HTML успешно опубликован на GitHub!");
    Ok(())
}