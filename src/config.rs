use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
pub struct Config {
    pub github: GithubConfig,
}

#[derive(Deserialize)]
pub struct GithubConfig {
    pub token: String,
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_content = fs::read_to_string("config.toml")?;
    let config: Config = toml::from_str(&config_content)?;
    Ok(config)
}