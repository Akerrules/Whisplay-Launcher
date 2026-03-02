use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone)]
pub struct IconData {
    pub rgba_64: Vec<u8>,
    pub rgba_80: Vec<u8>,
    pub rgba_96: Vec<u8>,
}

#[derive(Deserialize, Clone)]
pub struct AppConfig {
    pub name: String,
    pub script: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<[u8; 3]>,

    #[serde(skip)]
    pub resolved_script: PathBuf,
    #[serde(skip)]
    pub resolved_dir: PathBuf,
    #[serde(skip)]
    pub icon_data: Option<IconData>,
}

impl AppConfig {
    pub fn accent_color(&self) -> [u8; 3] {
        self.color.unwrap_or([29, 185, 84])
    }

    pub fn icon_type(&self) -> &str {
        self.icon.as_deref().unwrap_or("default")
    }
}

fn try_open_image(path: &Path) -> Result<image::DynamicImage, String> {
    // Try extension-based decoding first
    match image::open(path) {
        Ok(img) => return Ok(img),
        Err(e1) => {
            // Fall back to content-based format detection (magic bytes)
            match std::fs::read(path) {
                Ok(data) => match image::load_from_memory(&data) {
                    Ok(img) => return Ok(img),
                    Err(e2) => {
                        return Err(format!("{e1}; content fallback: {e2}"));
                    }
                },
                Err(e) => return Err(format!("{e1}; read: {e}")),
            }
        }
    }
}

fn load_icon(dir: &Path) -> Option<IconData> {
    for name in ["icon.ico", "icon.png", "icon.jpg", "icon.bmp"] {
        let path = dir.join(name);
        if !path.is_file() {
            continue;
        }
        match try_open_image(&path) {
            Ok(img) => {
                println!("    icon: loaded {} ({}x{})", path.display(), img.width(), img.height());
                let img64 = img.resize_exact(64, 64, image::imageops::FilterType::Lanczos3);
                let img80 = img.resize_exact(80, 80, image::imageops::FilterType::Lanczos3);
                let img96 = img.resize_exact(96, 96, image::imageops::FilterType::Lanczos3);
                return Some(IconData {
                    rgba_64: img64.to_rgba8().into_raw(),
                    rgba_80: img80.to_rgba8().into_raw(),
                    rgba_96: img96.to_rgba8().into_raw(),
                });
            }
            Err(e) => {
                eprintln!("    icon: failed to load {}: {e}", path.display());
            }
        }
    }
    None
}

pub fn load_apps(base_dir: &Path) -> Vec<AppConfig> {
    let cfg_path = base_dir.join("apps.json");
    let content = match std::fs::read_to_string(&cfg_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ERROR: Cannot read {}: {e}", cfg_path.display());
            return Vec::new();
        }
    };

    let entries: Vec<AppConfig> = match serde_json::from_str(&content) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("ERROR: Invalid apps.json: {e}");
            return Vec::new();
        }
    };

    let mut valid = Vec::new();
    for mut entry in entries {
        let raw_path = base_dir.join(&entry.script);
        match raw_path.canonicalize() {
            Ok(path) if path.is_file() => {
                let dir = path.parent().unwrap_or(base_dir).to_path_buf();
                entry.icon_data = load_icon(&dir);
                let icon_status = if entry.icon_data.is_some() { "+" } else { "-" };
                println!(
                    "  \u{2713} {} \u{2192} {} [icon:{icon_status}]",
                    entry.name,
                    path.display()
                );
                entry.resolved_dir = dir;
                entry.resolved_script = path;
                valid.push(entry);
            }
            _ => {
                eprintln!(
                    "  \u{2717} {} \u{2192} {} (not found)",
                    entry.name,
                    raw_path.display()
                );
            }
        }
    }

    valid
}

pub fn launch(app: &AppConfig, base_dir: &Path) {
    println!("\n{}", "=".repeat(50));
    println!("Launching: {}", app.name);
    println!("  Script:  {}", app.resolved_script.display());
    println!("{}\n", "=".repeat(50));

    let venv_python = app.resolved_dir.join("venv/bin/python3");
    let python = if venv_python.is_file() {
        venv_python
    } else {
        PathBuf::from("/usr/bin/python3")
    };

    let settings_dir = base_dir.join("settings");
    let pythonpath = match std::env::var("PYTHONPATH") {
        Ok(existing) => format!("{}:{existing}", settings_dir.display()),
        Err(_) => settings_dir.display().to_string(),
    };

    match Command::new(&python)
        .arg("-u")
        .arg(&app.resolved_script)
        .current_dir(&app.resolved_dir)
        .env("PYTHONPATH", &pythonpath)
        .status()
    {
        Ok(status) => println!("\nApp exited with {status}"),
        Err(e) => eprintln!("\nApp failed to start: {e}"),
    }
}
