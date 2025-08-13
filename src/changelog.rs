use crate::map::{MapEntry, MapError};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
enum ChangeType {
    Added,
    Modified,
    Deleted,
}

pub fn generate_changelog(old_entries: &[MapEntry], new_entries: &[MapEntry], output_dir: &Path) -> Result<(), MapError> {
    fs::create_dir_all(output_dir)?;
    let timestamp = chrono::Local::now().format("%d.%m.%Y");

    let mut html_content = format!(
        r#"<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="description" content="Изменения в файлах ассетов игры">
    <title>Патчноут {}</title>
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
            position: relative;
            overflow-x: hidden;
        }}
        body::before {{
            content: '';
            position: fixed;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            background-image: url('pattern_anti_spectrum.png');
            background-repeat: repeat;
            background-size: 200px;
            opacity: 0.03;
            pointer-events: none;
            z-index: 0;
        }}
        .changes {{
            width: 100%;
            flex: 1;
            position: relative;
            z-index: 1;
        }}
        .directory,
        .file,
        .path {{
            margin-left: 16px;
            width: 100%;
            position: relative;
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
        .lang-changes {{
            margin-top: 30px;
            padding: 20px;
            background: rgba(30, 30, 30, 0.7);
            border-radius: 8px;
            position: relative;
            z-index: 1;
        }}
        .diff-line {{
            font-family: 'Consolas', monospace;
            padding: 4px 8px;
            margin: 2px 0;
            border-radius: 4px;
            background: rgba(0, 0, 0, 0.2);
        }}
        .no-changes {{
            text-align: center;
            padding: 20px;
            color: #888;
            font-style: italic;
        }}
        .footer {{
            margin-top: 20px;
            text-align: center;
            padding: 10px;
            border-top: 1px solid #333;
            position: relative;
            z-index: 1;
        }}
        .footer a {{
            color: #c5c5c5;
            text-decoration: none;
            display: inline-flex;
            align-items: center;
            gap: 8px;
            transition: color 0.3s ease;
        }}
        .footer a:hover {{
            color: #8a9cff;
        }}
        .footer img {{
            width: 24px;
            height: 24px;
        }}
        h3 a {{
            color: #8a9cff;
            text-decoration: none;
            transition: color 0.3s ease;
        }}
        h3 a:hover {{
            color: #b39ddb;
        }}
    </style>
</head>
<body>
    <h1>Патчноут {}</h1>
    <h2>Изменения файловой структуры</h2>
    <h3>Источник: <a href="https://github.com/Art3mLapa" target="_blank">Krevetka</a></h3>
    <div class="changes">
"#,
        timestamp, timestamp
    );

    let mut changes: std::collections::BTreeMap<String, Vec<(String, ChangeType)>> = std::collections::BTreeMap::new();
    let old_map: std::collections::HashMap<_, _> = old_entries.iter().map(|e| (&e.path, &e.hash)).collect();
    let new_map: std::collections::HashMap<_, _> = new_entries.iter().map(|e| (&e.path, &e.hash)).collect();

    for (path, new_hash) in new_map.iter() {
        let change_type = match old_map.get(path) {
            Some(old_hash) if old_hash != new_hash => ChangeType::Modified,
            None => ChangeType::Added,
            _ => continue,
        };
        let (dir, file) = match path.rfind('/') {
            Some(idx) => (path[..idx].to_string(), path[idx + 1..].to_string()),
            None => (String::new(), path.to_string()),
        };
        changes.entry(dir).or_insert_with(Vec::new).push((file, change_type));
    }

    for path in old_map.keys() {
        if !new_map.contains_key(path) {
            let (dir, file) = match path.rfind('/') {
                Some(idx) => (path[..idx].to_string(), path[idx + 1..].to_string()),
                None => (String::new(), path.to_string()),
            };
            changes.entry(dir).or_insert_with(Vec::new).push((file, ChangeType::Deleted));
        }
    }

    let mut dir_tree: std::collections::BTreeMap<String, Vec<(String, String, ChangeType)>> = std::collections::BTreeMap::new();
    for (path, files) in changes {
        let parts = path.split('/').filter(|s| !s.is_empty()).map(String::from).collect::<Vec<_>>();
        let mut current_path = String::new();
        for part in &parts {
            let new_path = if current_path.is_empty() {
                part.to_string()
            } else {
                format!("{}/{}", current_path, part)
            };
            dir_tree.entry(new_path.clone()).or_insert_with(Vec::new);
            current_path = new_path;
        }
        if let Some(entries) = dir_tree.get_mut(&path) {
            entries.extend(files.iter().map(|(name, change_type)| (name.clone(), path.clone(), change_type.clone())));
        }
    }

    fn generate_html(
        path: &str,
        dir_tree: &std::collections::BTreeMap<String, Vec<(String, String, ChangeType)>>,
        html: &mut String,
        indent: usize,
    ) {
        let indent_str = " ".repeat(indent * 2);
        if !path.is_empty() {
            html.push_str(&format!(
                "{}<details class=\"directory\" open>\n{}  <summary class=\"name\">{}</summary>\n",
                indent_str,
                indent_str,
                path.split('/').last().unwrap_or(path)
            ));
            if let Some(files) = dir_tree.get(path) {
                if !files.is_empty() {
                    html.push_str(&format!("{}  <div class=\"path\">{}</div>\n", indent_str, path));
                }
            }
        }

        if let Some(files) = dir_tree.get(path) {
            for (name, _, change_type) in files {
                let (html_class, symbol) = match change_type {
                    ChangeType::Added => ("added", "+"),
                    ChangeType::Modified => ("modified", "~"),
                    ChangeType::Deleted => ("deleted", "-"),
                };
                html.push_str(&format!(
                    "{}  <div class=\"file {}\">\n{}    {} {}\n{}  </div>\n",
                    indent_str, html_class, indent_str, symbol, name, indent_str
                ));
            }
        }

        let current_prefix = if path.is_empty() { String::new() } else { format!("{}/", path) };
        let subdirs: Vec<_> = dir_tree
            .keys()
            .filter(|k| k.starts_with(&current_prefix) && *k != path && k[current_prefix.len()..].split('/').count() == 1)
            .collect();
        for subdir in subdirs {
            generate_html(subdir, dir_tree, html, if path.is_empty() { 0 } else { indent + 2 });
        }

        if !path.is_empty() {
            html.push_str(&format!("{}</details>\n", indent_str));
        }
    }

    let mut tree_html = String::new();
    generate_html("", &dir_tree, &mut tree_html, 0);
    html_content.push_str(&tree_html);

    html_content.push_str(
        r#"</div>
    <h2>Изменения в файле локализации</h2>
    <div class="lang-changes">
"#,
    );

    let diff_path = std::path::PathBuf::from("changes").join("lang_changes.diff");
    if diff_path.exists() {
        let diff_content = fs::read_to_string(&diff_path)?;
        for line in diff_content.lines() {
            let (class, content) = match line.chars().next() {
                Some('+') => ("added", &line[1..]),
                Some('-') => ("deleted", &line[1..]),
                Some('~') => ("modified", &line[1..]),
                _ => ("", line),
            };
            html_content.push_str(&format!(
                r#"<div class="diff-line {}">{}</div>"#,
                class,
                html_escape::encode_text(&content)
            ));
        }
    } else {
        html_content.push_str(r#"<div class="no-changes">Изменений в локализации не обнаружено</div>"#);
    }

    html_content.push_str(
        r#"</div>
    <div class="footer">
        <a href="https://github.com/BuildersSC/Krevetka" target="_blank">
            <img src="icon.png" alt="Krevetka Logo">
        </a>
    </div>
</body>
</html>"#,
    );

    fs::write(output_dir.join("index.html"), html_content)?;
    Ok(())
}
