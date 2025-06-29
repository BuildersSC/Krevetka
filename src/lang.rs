use crate::map::MapError;
use std::fs;
use std::path::Path;

pub fn process_lang_file(game_path: &Path, use_ots: bool) -> Result<(), MapError> {
    let standard_lang_path = game_path
        .join("runtime")
        .join("stalcraft")
        .join("modassets")
        .join("assets")
        .join("stalker")
        .join("lang")
        .join("ru.lang");
    let ots_lang_path = game_path
        .join("runtime")
        .join("stalcraft_ots")
        .join("modassets")
        .join("assets")
        .join("stalker")
        .join("lang")
        .join("ru.lang");

    let lang_path = if use_ots { &ots_lang_path } else { &standard_lang_path };
    if !lang_path.exists() {
        println!("Файл локализации не найден: {}", lang_path.display());
        return Ok(());
    }

    let env_dir = std::path::PathBuf::from("environment").join("lang");
    fs::create_dir_all(&env_dir)?;
    let env_lang = env_dir.join(if use_ots { "ru_ots.lang" } else { "ru.lang" });

    if !env_lang.exists() {
        fs::copy(&lang_path, &env_lang)?;
        println!("Создана первичная копия файла локализации");
        return Ok(());
    }

    let game_content = fs::read_to_string(&lang_path)?;
    let env_content = if use_ots {
        let standard_env_lang = env_dir.join("ru.lang");
        if standard_env_lang.exists() {
            fs::read_to_string(&standard_env_lang)?
        } else {
            String::new()
        }
    } else {
        fs::read_to_string(&env_lang)?
    };

    if game_content == env_content {
        return Ok(());
    }

    let game_lines: std::collections::HashMap<_, _> = game_content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let parts: Vec<_> = l.splitn(2, '=').collect();
            (parts[0].trim(), parts.get(1).map(|s| s.trim()))
        })
        .collect();

    let env_lines: std::collections::HashMap<_, _> = env_content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let parts: Vec<_> = l.splitn(2, '=').collect();
            (parts[0].trim(), parts.get(1).map(|s| s.trim()))
        })
        .collect();

    let mut diff_content = String::new();
    for (key, new_value) in &game_lines {
        match env_lines.get(key) {
            Some(old_value) if old_value != new_value => {
                diff_content.push_str(&format!("~{} = {}\n", key, new_value.unwrap_or("")));
            }
            None => {
                diff_content.push_str(&format!("+{} = {}\n", key, new_value.unwrap_or("")));
            }
            _ => {}
        }
    }
    for key in env_lines.keys() {
        if !game_lines.contains_key(key) {
            if let Some(old_value) = env_lines.get(key).and_then(|v| *v) {
                diff_content.push_str(&format!("-{} = {}\n", key, old_value));
            } else {
                diff_content.push_str(&format!("-{}\n", key));
            }
        }
    }

    if !diff_content.is_empty() {
        let diff_path = std::path::PathBuf::from("changes").join("lang_changes.diff");
        if let Some(parent) = diff_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&diff_path, diff_content)?;
        fs::copy(&lang_path, &env_lang)?;
        println!("Обнаружены и сохранены изменения в файле локализации");
    }

    Ok(())
}