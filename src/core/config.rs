use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn get_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/default".to_string());
    Path::new(&home)
        .join("Library")
        .join("Application Support")
        .join("language-handler")
        .join("config.json")
}

fn create_default_config() -> HashMap<String, String> {
    let mut config = HashMap::new();
    config.insert("Terminal".to_string(), "US".to_string());
    config.insert("iTerm2".to_string(), "US".to_string());
    config.insert("iTerm".to_string(), "US".to_string());
    config.insert("Code".to_string(), "US".to_string());
    config.insert("Visual Studio Code".to_string(), "US".to_string());
    config.insert("Xcode".to_string(), "US".to_string());
    config
}

pub fn load_or_create_config() -> HashMap<String, String> {
    let config_path = get_config_path();

    if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                println!("Warning: Could not create config directory: {}", e);
                return create_default_config();
            }
        }
    }

    if config_path.exists() {
        match fs::read_to_string(&config_path) {
            Ok(content) => match serde_json::from_str::<HashMap<String, String>>(&content) {
                Ok(config) => {
                    println!("Loaded configuration from: {}", config_path.display());
                    println!("Switching rules:");
                    for (app, layout) in &config {
                        println!("  {} -> {}", app, layout);
                    }
                    return config;
                }
                Err(e) => {
                    println!("Error parsing JSON config: {}. Creating a default one.", e);
                }
            },
            Err(e) => {
                println!("Error reading config file: {}. Creating a default one.", e);
            }
        }
    }

    let default_config = create_default_config();

    match serde_json::to_string_pretty(&default_config) {
        Ok(json_content) => {
            if let Err(e) = fs::write(&config_path, json_content) {
                println!("Warning: Could not save default config: {}", e);
            } else {
                println!(
                    "Created default configuration file at: {}",
                    config_path.display()
                );
                println!("Default switching rules:");
                for (app, layout) in &default_config {
                    println!("  {} -> {}", app, layout);
                }
            }
        }
        Err(e) => {
            println!("Error serializing default config: {}", e);
        }
    }

    default_config
}
