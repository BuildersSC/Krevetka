use std::fs::{self, File};
use std::io::{self, Read};
use std::path::PathBuf;
use thiserror::Error;
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

#[derive(Error, Debug)]
pub enum MapError {
    #[error("Ошибка чтения реестра: {0}")]
    RegistryError(String),
    #[error("Ошибка ввода/вывода: {0}")]
    IoError(#[from] io::Error),
    #[error("Файл игры не найден")]
    GameFileNotFound,
    #[error("Ошибка чтения структуры файла: {0}")]
    ParseError(String),
    #[error("Некорректный формат файла: {0}")]
    InvalidFormat(String),
    #[error("Ошибка конфигурации: {0}")]
    ConfigError(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapEntry {
    pub path: String,
    pub hash: Vec<u8>,
}

impl MapEntry {
    pub fn read_from(file: &mut File) -> Result<Self, MapError> {
        let mut size_buf = [0u8; 2];
        file.read_exact(&mut size_buf)?;
        let size = u16::from_be_bytes(size_buf);

        if size == 0 || size > 1024 {
            return Err(MapError::InvalidFormat(format!(
                "Некорректный размер пути: {} байт",
                size
            )));
        }

        let mut path_buf = vec![0u8; size as usize];
        file.read_exact(&mut path_buf)?;
        let path = String::from_utf8(path_buf)
            .map_err(|e| MapError::ParseError(format!("Некорректная UTF-8 последовательность: {}", e)))?;

        let mut hash = vec![0u8; 20];
        file.read_exact(&mut hash)?;

        Ok(MapEntry { path, hash })
    }
}

pub fn get_game_path() -> Result<PathBuf, MapError> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let exbo_key = hkcu
        .open_subkey("SOFTWARE\\EXBO")
        .map_err(|e| MapError::RegistryError(e.to_string()))?;
    let root_path: String = exbo_key
        .get_value("root")
        .map_err(|e| MapError::RegistryError(e.to_string()))?;
    Ok(PathBuf::from(root_path))
}

pub fn get_stalcraft_map_path() -> Result<PathBuf, MapError> {
    let game_path = get_game_path()?;
    Ok(game_path.join("runtime").join("stalcraft.map"))
}

pub fn init_environment() -> Result<PathBuf, MapError> {
    let env_dir = PathBuf::from("environment");
    fs::create_dir_all(&env_dir)?;

    let env_map = env_dir.join("stalcraft.map");
    if !env_map.exists() {
        let game_map = get_stalcraft_map_path()?;
        fs::copy(&game_map, &env_map)?;
    }
    Ok(env_map)
}

pub fn read_map_entries(file_path: &std::path::Path) -> Result<Vec<MapEntry>, MapError> {
    let mut file = File::open(file_path)?;
    let file_size = file.metadata()?.len();
    if file_size < 4 {
        return Err(MapError::InvalidFormat("Файл слишком мал".to_string()));
    }

    let mut count_buf = [0u8; 4];
    file.read_exact(&mut count_buf)?;
    let count = u32::from_be_bytes(count_buf);

    let mut entries = Vec::with_capacity(count as usize);
    for i in 0..count {
        entries.push(MapEntry::read_from(&mut file).map_err(|e| {
            MapError::InvalidFormat(format!("Ошибка чтения записи {}/{}: {}", i + 1, count, e))
        })?);
    }
    Ok(entries)
}