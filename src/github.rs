use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PublishError {
    #[error("Ошибка ввода/вывода: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Ошибка выполнения BunJS: {0}")]
    ExecutionError(String),
    #[error("Ошибка загрузки переменных окружения: {0}")]
    EnvError(#[from] dotenvy::Error),
}

pub fn publish_html() -> Result<(), PublishError> {
    // Загрузка переменных из .env файла
    dotenvy::dotenv()?;

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