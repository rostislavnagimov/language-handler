use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn get_plist_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/default".to_string());
    Path::new(&home)
        .join("Library")
        .join("LaunchAgents")
        .join("com.language-handler.plist")
}

pub fn get_binary_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let current_exe = std::env::current_exe()?;
    Ok(current_exe)
}

fn get_log_directory() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/default".to_string());
    Path::new(&home)
        .join("Library")
        .join("Logs")
        .join("language-handler")
}

fn create_plist_content(binary_path: &str) -> String {
    let log_dir = get_log_directory();
    let stdout_log = log_dir.join("language-handler.log");
    let stderr_log = log_dir.join("language-handler.error.log");

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.language-handler</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{}</string>
    <key>StandardErrorPath</key>
    <string>{}</string>
    <key>ProcessType</key>
    <string>Interactive</string>
    <key>SoftResourceLimits</key>
    <dict>
        <key>NumberOfFiles</key>
        <integer>256</integer>
    </dict>
</dict>
</plist>"#,
        binary_path,
        stdout_log.to_string_lossy(),
        stderr_log.to_string_lossy()
    )
}

pub fn install_service() -> Result<(), Box<dyn std::error::Error>> {
    let binary_path = get_binary_path()?;
    let plist_path = get_plist_path();
    let log_dir = get_log_directory();

    if let Some(parent) = plist_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::create_dir_all(&log_dir)?;

    let plist_content = create_plist_content(binary_path.to_str().unwrap());
    fs::write(&plist_path, plist_content)?;

    let output = Command::new("launchctl")
        .args(&["load", plist_path.to_str().unwrap()])
        .output()?;

    if output.status.success() {
        println!("✅ Service installed successfully!");
        println!("Binary path: {}", binary_path.display());
        println!("Plist path: {}", plist_path.display());
        println!("Logs directory: {}", log_dir.display());
        println!("\nThe service will start automatically on next login.");
        println!("To start it now, run: launchctl start com.language-handler");

        install_log_rotation()?;
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to load service: {}", error).into());
    }

    Ok(())
}

pub fn uninstall_service() -> Result<(), Box<dyn std::error::Error>> {
    let plist_path = get_plist_path();

    if !plist_path.exists() {
        println!("Service is not installed.");
        return Ok(());
    }

    let output = Command::new("launchctl")
        .args(&["unload", plist_path.to_str().unwrap()])
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        println!("Warning: Failed to unload service: {}", error);
    }

    fs::remove_file(&plist_path)?;

    println!("✅ Service uninstalled successfully!");
    Ok(())
}

pub fn check_service_status() -> Result<(), Box<dyn std::error::Error>> {
    let plist_path = get_plist_path();

    if !plist_path.exists() {
        println!("❌ Service is not installed.");
        return Ok(());
    }

    let output = Command::new("launchctl")
        .args(&["list", "com.language-handler"])
        .output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            println!("📋 Service is installed but not running.");
        } else {
            println!("✅ Service is installed and running.");
            println!("Details:\n{}", stdout);
        }
    } else {
        println!("📋 Service is installed but not loaded.");
    }

    println!("Plist location: {}", plist_path.display());
    Ok(())
}

fn install_log_rotation() -> Result<(), Box<dyn std::error::Error>> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/default".to_string());
    let log_rotation_plist = Path::new(&home)
        .join("Library")
        .join("LaunchAgents")
        .join("com.language-handler.logrotate.plist");

    let log_dir = get_log_directory();
    let script_content = format!(
        r#"#!/bin/bash

LOG_DIR="{}"
MAX_SIZE=10485760 # 10MB in bytes
MAX_DAYS=7

rotate_log() {{
    local logfile="$1"

    if [[ -f "$logfile" ]]; then
        local size=$(stat -f%z "$logfile" 2>/dev/null || echo 0)

        if [[ $size -gt $MAX_SIZE ]]; then
            echo "$(date): Rotating $logfile (size: $size bytes)"

            tail -n 1000 "$logfile" > "${{logfile}}.tmp"
            mv "${{logfile}}.tmp" "$logfile"

            echo "$(date): Log rotated, kept last 1000 lines" >> "$logfile"
        fi
    fi
}}

find "$LOG_DIR" -name "*.log.*" -mtime +$MAX_DAYS -delete 2>/dev/null

rotate_log "$LOG_DIR/language-handler.log"
rotate_log "$LOG_DIR/language-handler.error.log"

exit 0
"#,
        log_dir.to_string_lossy()
    );

    let script_path = log_dir.join("rotate_logs.sh");
    fs::write(&script_path, script_content)?;

    Command::new("chmod")
        .args(&["+x", script_path.to_str().unwrap()])
        .output()?;

    let rotation_plist_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.language-handler.logrotate</string>
    <key>ProgramArguments</key>
    <array>
        <string>/bin/bash</string>
        <string>{}</string>
    </array>
    <key>StartCalendarInterval</key>
    <dict>
        <key>Hour</key>
        <integer>2</integer>
        <key>Minute</key>
        <integer>0</integer>
    </dict>
    <key>StandardOutPath</key>
    <string>{}/rotation.log</string>
    <key>StandardErrorPath</key>
    <string>{}/rotation.error.log</string>
</dict>
</plist>"#,
        script_path.to_string_lossy(),
        log_dir.to_string_lossy(),
        log_dir.to_string_lossy()
    );

    fs::write(&log_rotation_plist, rotation_plist_content)?;

    let output = Command::new("launchctl")
        .args(&["load", log_rotation_plist.to_str().unwrap()])
        .output()?;

    if output.status.success() {
        println!("📋 Log rotation task installed (runs daily at 2:00 AM)");
    } else {
        println!("⚠️  Warning: Could not install log rotation task");
    }

    Ok(())
}

pub fn show_logs() -> Result<(), Box<dyn std::error::Error>> {
    let log_dir = get_log_directory();
    let main_log = log_dir.join("language-handler.log");
    let error_log = log_dir.join("language-handler.error.log");

    println!("📂 Logs location: {}", log_dir.display());
    println!();

    if main_log.exists() {
        if let Ok(metadata) = fs::metadata(&main_log) {
            println!("📄 Main log: {} ({:.2} MB)",
                main_log.display(),
                metadata.len() as f64 / 1024.0 / 1024.0
            );
        }

        println!("Last 20 lines:");
        let output = Command::new("tail")
            .args(&["-n", "20", main_log.to_str().unwrap()])
            .output()?;

        if output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
    }

    if error_log.exists() {
        if let Ok(metadata) = fs::metadata(&error_log) {
            println!("🚨 Error log: {} ({:.2} MB)",
                error_log.display(),
                metadata.len() as f64 / 1024.0 / 1024.0
            );
        }
    }

    println!("\n💡 Commands:");
    println!("  View logs: tail -f {}", main_log.to_string_lossy());

    Ok(())
}
