use language_handler::core::service;
use language_handler::run;
use std::env;

fn print_help() {
    println!("Language Handler - Automatic keyboard layout switcher for macOS");
    println!();
    println!("USAGE:");
    println!("    language-handler [COMMAND]");
    println!();
    println!("COMMANDS:");
    println!("    run         Run the language handler (default)");
    println!("    install     Install as a system service (auto-start on login)");
    println!("    uninstall   Remove the system service");
    println!("    status      Check service installation status");
    println!("    logs        Show recent logs and log file locations");
    println!("    help        Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("    language-handler                  # Run normally");
    println!("    language-handler install          # Install as service");
    println!("    language-handler status           # Check if service is installed");
    println!("    language-handler logs             # View recent logs");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let command = if args.len() > 1 {
        args[1].as_str()
    } else {
        "run"
    };

    match command {
        "install" => {
            if let Err(e) = service::install_service() {
                eprintln!("Error installing service: {}", e);
                std::process::exit(1);
            }
        },
        "uninstall" => {
            if let Err(e) = service::uninstall_service() {
                eprintln!("Error uninstalling service: {}", e);
                std::process::exit(1);
            }
        },
        "status" => {
            if let Err(e) = service::check_service_status() {
                eprintln!("Error checking service status: {}", e);
                std::process::exit(1);
            }
        },
        "logs" => {
            if let Err(e) = service::show_logs() {
                eprintln!("Error showing logs: {}", e);
                std::process::exit(1);
            }
        },
        "help" | "--help" | "-h" => {
            print_help();
        },
        "run" => {
            run();
        },
        _ => {
            eprintln!("Unknown command: {}", command);
            eprintln!("Use 'language-handler help' for usage information.");
            std::process::exit(1);
        }
    }
}
