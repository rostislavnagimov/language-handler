use std::path::PathBuf;

pub fn get_config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/default".to_string());
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("language-handler")
}

pub fn get_config_path() -> PathBuf {
    get_config_dir().join("config.json")
}

pub fn get_launchagent_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/default".to_string());
    PathBuf::from(home)
        .join("Library")
        .join("LaunchAgents")
        .join("com.languagehandler.daemon.plist")
}
