use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};
use chrono::Local;
use thiserror::Error;
use winreg::{RegKey, enums::HKEY_CURRENT_USER};

#[derive(Error, Debug)]
pub enum MapError {
    #[error("Ошибка чтения реестра: {0}")]
    RegistryError(String),
    #[error("Ошибка ввода/вывода: {0}")]
    IoError(#[from] io::Error),
    #[error("Файл игры не найден")]
    GameFileNotFound,
    #[error("Превышено время ожидания файла игры")]
    GameFileTimeout,
    #[error("Ошибка чтения структуры файла: {0}")]
    ParseError(String),
    #[error("Некорректный формат файла: {0}")]
    InvalidFormat(String),
}

type Result<T> = std::result::Result<T, MapError>;

#[derive(Debug, Clone, PartialEq)]
struct MapEntry {
    path: String,
    hash: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
enum ChangeType {
    Added,
    Modified,
    Deleted,
}

impl MapEntry {
    fn read_from(file: &mut File) -> Result<Self> {
        // Проверяем, что можем прочитать как минимум 2 байта для размера
        let mut size_buf = [0u8; 2];
        match file.read_exact(&mut size_buf) {
            Ok(_) => {},
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                return Err(MapError::InvalidFormat("Неожиданный конец файла при чтении размера пути".to_string()));
            },
            Err(e) => return Err(MapError::IoError(e)),
        }

        let size = u16::from_be_bytes(size_buf);
        
        // Проверяем, что размер пути разумный
        if size == 0 {
            return Err(MapError::InvalidFormat("Нулевой размер пути".to_string()));
        }
        if size > 1024 {
            return Err(MapError::InvalidFormat(format!("Слишком большой размер пути: {} байт", size)));
        }

        let mut path_buf = vec![0u8; size as usize];
        match file.read_exact(&mut path_buf) {
            Ok(_) => {},
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                return Err(MapError::InvalidFormat("Неожиданный конец файла при чтении пути".to_string()));
            },
            Err(e) => return Err(MapError::IoError(e)),
        }

        let path = String::from_utf8(path_buf)
            .map_err(|e| MapError::ParseError(format!("Некорректная UTF-8 последовательность в пути: {}", e)))?;

        let mut hash = vec![0u8; 20];
        match file.read_exact(&mut hash) {
            Ok(_) => {},
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                return Err(MapError::InvalidFormat("Неожиданный конец файла при чтении хеша".to_string()));
            },
            Err(e) => return Err(MapError::IoError(e)),
        }

        Ok(MapEntry { path, hash })
    }
}

fn split_path(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

fn generate_directory_tree(changes: &std::collections::BTreeMap<String, Vec<(String, ChangeType)>>) -> Result<String> {
    let mut html_content = String::new();
    
    // Создаем дерево директорий
    let mut dir_tree: std::collections::BTreeMap<String, Vec<(String, String, ChangeType)>> = std::collections::BTreeMap::new();
    
    for (path, files) in changes {
        let parts = split_path(path);
        let mut current_path = String::new();
        
        // Добавляем каждый уровень пути в дерево
        for part in &parts {
            let new_path = if current_path.is_empty() {
                part.to_string()
            } else {
                format!("{}/{}", current_path, part)
            };
            
            if !dir_tree.contains_key(&new_path) {
                dir_tree.insert(new_path.clone(), Vec::new());
            }
            current_path = new_path;
        }
        
        // Добавляем файлы в последнюю директорию
        if let Some(entries) = dir_tree.get_mut(path) {
            entries.extend(files.iter().map(|(name, change_type)| 
                (name.clone(), path.clone(), change_type.clone())));
        }
    }
    
    // Рекурсивная функция для генерации HTML
    fn generate_html(
        path: &str,
        dir_tree: &std::collections::BTreeMap<String, Vec<(String, String, ChangeType)>>,
        html: &mut String,
        indent: usize
    ) {
        let indent_str = " ".repeat(indent * 2);
        
        // Пропускаем корневую директорию
        if !path.is_empty() {
            // Открываем details для директории
            html.push_str(&format!("{}  <details class=\"directory\" open>\n", indent_str));
            html.push_str(&format!("{}    <summary class=\"name\">{}</summary>\n", indent_str, 
                path.split('/').last().unwrap_or(path)));
            
            // Добавляем полный путь только если в директории есть измененные файлы
            if let Some(files) = dir_tree.get(path) {
                if !files.is_empty() {
                    html.push_str(&format!("{}    <div class=\"path\">{}</div>\n", indent_str, path));
                }
            }
        }
        
        // Добавляем файлы текущей директории
        if let Some(files) = dir_tree.get(path) {
            for (name, _, change_type) in files {
                let (html_class, symbol) = match change_type {
                    ChangeType::Added => ("added", "+"),
                    ChangeType::Modified => ("modified", "~"),
                    ChangeType::Deleted => ("deleted", "-"),
                };
                
                html.push_str(&format!("{}    <div class=\"file {}\">\n{}      {} {}\n{}    </div>\n",
                    indent_str, html_class, indent_str, symbol, name, indent_str));
            }
        }
        
        // Рекурсивно обрабатываем поддиректории
        let current_prefix = if path.is_empty() { String::new() } else { format!("{}/", path) };
        let subdirs: Vec<_> = dir_tree.keys()
            .filter(|k| k.starts_with(&current_prefix) && *k != path && 
                   k[current_prefix.len()..].split('/').count() == 1)
            .collect();
        
        for subdir in subdirs {
            generate_html(subdir, dir_tree, html, if path.is_empty() { 0 } else { indent + 2 });
        }
        
        if !path.is_empty() {
            html.push_str(&format!("{}  </details>\n", indent_str));
        }
    }
    
    // Генерируем дерево
    generate_html("", &dir_tree, &mut html_content, 0);
    
    Ok(html_content)
}

fn process_lang_file(game_path: &Path) -> Result<()> {
    let lang_path = game_path.parent().unwrap() // runtime
        .parent().unwrap() // EXBO
        .parent().unwrap() // Roaming
        .parent().unwrap() // AppData
        .parent().unwrap() // User folder
        .join("AppData")
        .join("Roaming")
        .join("EXBO")
        .join("runtime")
        .join("stalcraft")
        .join("modassets")
        .join("assets")
        .join("stalker")
        .join("lang")
        .join("ru.lang");

    // Проверяем существование исходного файла
    if !lang_path.exists() {
        println!("Файл локализации не найден по пути: {}", lang_path.display());
        return Ok(());
    }

    let env_dir = PathBuf::from("environment").join("lang");
    
    // Создаем директорию с обработкой ошибок
    if let Err(e) = fs::create_dir_all(&env_dir) {
        return Err(MapError::IoError(io::Error::new(
            io::ErrorKind::Other,
            format!("Не удалось создать директорию для языковых файлов: {}", e)
        )));
    }
    
    let env_lang = env_dir.join("ru.lang");
    
    // Копируем файл, если его нет в окружении
    if !env_lang.exists() {
        match fs::copy(&lang_path, &env_lang) {
            Ok(_) => {
                println!("Создана первичная копия файла локализации");
                return Ok(());
            },
            Err(e) => {
                return Err(MapError::IoError(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Не удалось скопировать файл локализации: {}", e)
                )));
            }
        }
    }
    
    // Читаем оба файла с обработкой ошибок
    let game_content = match fs::read_to_string(&lang_path) {
        Ok(content) => content,
        Err(e) => {
            return Err(MapError::IoError(io::Error::new(
                io::ErrorKind::Other,
                format!("Не удалось прочитать файл игры: {}", e)
            )));
        }
    };

    let env_content = match fs::read_to_string(&env_lang) {
        Ok(content) => content,
        Err(e) => {
            return Err(MapError::IoError(io::Error::new(
                io::ErrorKind::Other,
                format!("Не удалось прочитать локальный файл: {}", e)
            )));
        }
    };
    
    if game_content == env_content {
        return Ok(());
    }
    
    // Сравниваем строки
    let game_lines: std::collections::HashMap<_, _> = game_content.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let parts: Vec<_> = l.splitn(2, '=').collect();
            (parts[0].trim(), parts.get(1).map(|s| s.trim()))
        })
        .collect();
    
    let env_lines: std::collections::HashMap<_, _> = env_content.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let parts: Vec<_> = l.splitn(2, '=').collect();
            (parts[0].trim(), parts.get(1).map(|s| s.trim()))
        })
        .collect();
    
    let timestamp = Local::now().format("%d_%m_%Y");
    let mut diff_content = String::new();
    
    // Находим изменения
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
    
    // Находим удаленные строки
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
        let diff_path = PathBuf::from("changes").join(format!("lang_changes_{}.diff", timestamp));
        
        // Создаем директорию для изменений с обработкой ошибок
        if let Err(e) = fs::create_dir_all(diff_path.parent().unwrap()) {
            return Err(MapError::IoError(io::Error::new(
                io::ErrorKind::Other,
                format!("Не удалось создать директорию для изменений: {}", e)
            )));
        }

        // Записываем файл изменений
        if let Err(e) = fs::write(&diff_path, diff_content) {
            return Err(MapError::IoError(io::Error::new(
                io::ErrorKind::Other,
                format!("Не удалось записать файл изменений: {}", e)
            )));
        }

        // Копируем обновленный файл
        if let Err(e) = fs::copy(&lang_path, &env_lang) {
            return Err(MapError::IoError(io::Error::new(
                io::ErrorKind::Other,
                format!("Не удалось обновить локальную копию файла: {}", e)
            )));
        }

        println!("Обнаружены и сохранены изменения в файле локализации");
    }
    
    Ok(())
}

fn generate_changelog(old_entries: &[MapEntry], new_entries: &[MapEntry], output_dir: &Path) -> Result<()> {
    fs::create_dir_all(output_dir)?;
    
    let timestamp = Local::now().format("%d.%m.%Y");
    let mut html_content = format!(
        r#"<!DOCTYPE html>
<html lang="ru">
<head>
    <title>Патчноут</title>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <meta name="description" content="Изменения в файлах ассетов игры" />
    <style>
        body {{
            background-color: #1e1e1e;
            color: #c5c5c5;
            font-family: monospace;
            padding: 16px;
            width: 100%;
            min-height: 100vh;
            display: flex;
            flex-direction: column;
        }}
        .changes {{
            width: 100%;
            flex: 1;
        }}
        .directory,
        .file,
        .path {{
            margin-left: 16px;
            width: 100%;
        }}
        .path {{
            opacity: 0.5;
        }}
        .directory > .name {{
            font-size: 16px;
        }}
        .added {{ color: #a0d468; }}
        .deleted {{ color: #ff6b6b; }}
        .modified {{ color: #ffd700; }}
        .footer {{
            margin-top: 20px;
            text-align: center;
            padding: 10px;
            border-top: 1px solid #333;
        }}
        .footer a {{
            color: #c5c5c5;
            text-decoration: none;
            display: inline-flex;
            align-items: center;
            gap: 8px;
        }}
        .footer img {{
            width: 24px;
            height: 24px;
        }}
    </style>
</head>
<body>
    <h1>ChangeLog {}</h1>
    <h2>Изменения файловой структуры ассетов игры</h2>
    <h3>Источник: <a href="https://t.me/github.com/Art3mLapa" target="_blank">Krevetka</a></h3>
    <div class="changes">
    "#, timestamp);

    // Создаем структуру изменений по директориям
    let mut changes: std::collections::BTreeMap<String, Vec<(String, ChangeType)>> = std::collections::BTreeMap::new();

    // Создаем HashMap для быстрого поиска
    let old_map: std::collections::HashMap<_, _> = old_entries
        .iter()
        .map(|e| (&e.path, &e.hash))
        .collect();
    
    let new_map: std::collections::HashMap<_, _> = new_entries
        .iter()
        .map(|e| (&e.path, &e.hash))
        .collect();

    // Поиск изменений и группировка по директориям
    for (path, new_hash) in new_map.iter() {
        let change_type = match old_map.get(path) {
            Some(old_hash) if old_hash != new_hash => ChangeType::Modified,
            None => ChangeType::Added,
            _ => continue,
        };

        let (dir, file) = match path.rfind('/') {
            Some(idx) => (path[..idx].to_string(), path[idx+1..].to_string()),
            None => (String::new(), path.to_string()),
        };

        changes.entry(dir)
            .or_insert_with(Vec::new)
            .push((file, change_type));
    }

    // Добавляем удаленные файлы
    for path in old_map.keys() {
        if !new_map.contains_key(path) {
            let (dir, file) = match path.rfind('/') {
                Some(idx) => (path[..idx].to_string(), path[idx+1..].to_string()),
                None => (String::new(), path.to_string()),
            };

            changes.entry(dir)
                .or_insert_with(Vec::new)
                .push((file, ChangeType::Deleted));
        }
    }

    // Генерируем HTML с новой структурой
    let tree_html = generate_directory_tree(&changes)?;
    html_content.push_str(&tree_html);
    html_content.push_str(r#"</div>
    <div class="footer">
        <a href="https://github.com/BuildersSC/Krevetka" target="_blank">
            <img src="@icon.png" alt="Krevetka Logo">
            <span>This HTML site made by Krevetka.</span>
        </a>
    </div>
</body>
</html>"#);

    // Записываем HTML файл
    let timestamp_str = timestamp.to_string().replace(".", "_");
    fs::write(
        output_dir.join(format!("changelog_{}.html", timestamp_str)),
        html_content,
    )?;

    Ok(())
}

fn get_game_path() -> Result<PathBuf> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let exbo_key = hkcu.open_subkey("SOFTWARE\\EXBO")
        .map_err(|e| MapError::RegistryError(e.to_string()))?;
    let root_path: String = exbo_key.get_value("root")
        .map_err(|e| MapError::RegistryError(e.to_string()))?;
    Ok(PathBuf::from(root_path).join("runtime").join("stalcraft.map"))
}

fn init_environment() -> Result<PathBuf> {
    let env_dir = PathBuf::from("environment");
    fs::create_dir_all(&env_dir)?;
    
    let env_map = env_dir.join("stalcraft.map");
    if !env_map.exists() {
        let game_map = get_game_path()?;
        fs::copy(&game_map, &env_map)?;
    }
    
    Ok(env_map)
}

fn read_map_entries(file_path: &Path) -> Result<Vec<MapEntry>> {
    let mut file = File::open(file_path)?;
    
    // Проверяем размер файла
    let file_size = file.metadata()?.len();
    if file_size < 4 {
        return Err(MapError::InvalidFormat("Файл слишком мал для корректной структуры".to_string()));
    }

    // Читаем количество записей
    let mut count_buf = [0u8; 4];
    file.read_exact(&mut count_buf)
        .map_err(|e| MapError::IoError(io::Error::new(io::ErrorKind::Other,
            format!("Не удалось прочитать количество записей: {}", e))))?;
    
    let count = u32::from_be_bytes(count_buf);
    
    // Проверяем, что количество записей разумное
    if count == 0 {
        return Err(MapError::InvalidFormat("Файл не содержит записей".to_string()));
    }
    
    let approx_entry_size = 22; // минимальный размер записи (2 байта размер + 20 байт хеш)
    if (count as u64) * (approx_entry_size as u64) > file_size {
        return Err(MapError::InvalidFormat(
            format!("Некорректное количество записей: {}. Файл слишком мал для такого количества", count)
        ));
    }

    let mut entries = Vec::with_capacity(count as usize);
    for i in 0..count {
        match MapEntry::read_from(&mut file) {
            Ok(entry) => entries.push(entry),
            Err(e) => return Err(MapError::InvalidFormat(
                format!("Ошибка чтения записи {}/{}: {}", i + 1, count, e)
            )),
        }
    }

    Ok(entries)
}

fn monitor_changes() -> Result<()> {
    let env_map = init_environment()?;
    let mut last_check = Instant::now();
    let timeout_duration = Duration::from_secs(30);
    
    loop {
        let game_map_result = get_game_path().and_then(|path| {
            if path.exists() {
                Ok(path)
            } else {
                Err(MapError::GameFileNotFound)
            }
        });

        match game_map_result {
            Ok(game_map) => {
                // Проверяем изменения в map файле
                let game_len = fs::metadata(&game_map)?.len();
                let env_len = fs::metadata(&env_map)?.len();

                if game_len != env_len {
                    println!("Обнаружены изменения в файле карты!");
                    let old_entries = read_map_entries(&env_map)?;
                    let new_entries = read_map_entries(&game_map)?;
                    generate_changelog(&old_entries, &new_entries, Path::new("changes"))?;
                    fs::copy(&game_map, &env_map)?;
                    println!("Изменения сохранены в директории 'changes'");
                }

                // Проверяем изменения в lang файле
                if let Err(e) = process_lang_file(&game_map) {
                    eprintln!("Ошибка при обработке lang файла: {}", e);
                }
                
                last_check = Instant::now();
                thread::sleep(Duration::from_secs(5));
            }
            Err(MapError::GameFileNotFound) => {
                if last_check.elapsed() > timeout_duration {
                    return Err(MapError::GameFileTimeout);
                }
                println!("Файл игры не найден, повторная попытка через 1 секунду...");
                thread::sleep(Duration::from_secs(1));
            }
            Err(e) => return Err(e),
        }
    }
}

fn main() {
    match monitor_changes() {
        Ok(_) => println!("Программа завершена успешно"),
        Err(e) => eprintln!("Ошибка: {}", e),
    }
} 