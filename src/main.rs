use std::thread;
use std::time::Duration;
use crate::changelog::generate_changelog;
use crate::github::publish_html;
use crate::lang::process_lang_file;
use crate::map::{get_game_path, get_stalcraft_map_path, init_environment, read_map_entries, MapError};

mod changelog;
mod github;
mod lang;
mod map;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Инициализация окружения
    let env_map = init_environment()?;

    // Основной цикл мониторинга
    let mut last_diff_content = String::new();
    loop {
        let game_map_result = get_stalcraft_map_path().and_then(|path| {
            if path.exists() {
                Ok(path)
            } else {
                Err(MapError::GameFileNotFound)
            }
        });

        match game_map_result {
            Ok(game_map) => {
                let mut changes_detected = false;
                let mut map_entries = None;

                // Проверка изменений в файле карты
                let game_len = std::fs::metadata(&game_map)?.len();
                let env_len = std::fs::metadata(&env_map)?.len();

                if game_len != env_len {
                    println!("Обнаружены изменения в файле карты!");
                    let old_entries = read_map_entries(&env_map)?;
                    let new_entries = read_map_entries(&game_map)?;
                    map_entries = Some((old_entries, new_entries));
                    std::fs::copy(&game_map, &env_map)?;
                    changes_detected = true;
                    println!("Изменения в файле карты сохранены");
                }

                // Проверка изменений в файле локализации
                if let Ok(game_dir) = get_game_path() {
                    if let Err(e) = process_lang_file(&game_dir) {
                        eprintln!("Ошибка при обработке lang файла: {}", e);
                    } else {
                        let diff_path = std::path::PathBuf::from("changes").join("lang_changes.diff");
                        if diff_path.exists() {
                            match std::fs::read_to_string(&diff_path) {
                                Ok(current_diff_content) => {
                                    if current_diff_content != last_diff_content {
                                        changes_detected = true;
                                        last_diff_content = current_diff_content;
                                    }
                                }
                                Err(e) => eprintln!("Ошибка при чтении diff файла: {}", e),
                            }
                        }
                    }
                }

                // Генерация и публикация ChangeLog, если есть изменения
                if changes_detected {
                    let entries = map_entries.unwrap_or_else(|| {
                        let entries = read_map_entries(&env_map).expect("Не удалось прочитать env_map");
                        (entries.clone(), entries)
                    });
                    generate_changelog(&entries.0, &entries.1, std::path::Path::new("docs"))?;
                    publish_html()?;
                    println!("Изменения сохранены в HTML документе и опубликованы");
                }

                thread::sleep(Duration::from_secs(1));
            }
            Err(MapError::GameFileNotFound) => {
                println!("Файл игры не найден, повторная попытка через 1 секунду...");
                thread::sleep(Duration::from_secs(1));
            }
            Err(e) => {
                eprintln!("Ошибка при получении пути к файлу: {}", e);
                thread::sleep(Duration::from_secs(1));
            }
        }
    }
}