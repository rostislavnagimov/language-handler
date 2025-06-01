use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicyProhibited};
use cocoa::base::nil;
use cocoa::foundation::NSAutoreleasePool;

pub mod core;

// Re-export service functions for convenience
pub use core::service::{install_service, uninstall_service, check_service_status};

pub(crate) mod state {
    use std::collections::HashMap;

    pub(crate) static mut CURRENT_APP: Option<String> = None;
    pub(crate) static mut CURRENT_KEYBOARD_LAYOUT: Option<String> = None;
    pub(crate) static mut APP_LAYOUT_RULES: Option<HashMap<String, String>> = None;
}

pub fn run() {
    let config = core::config::load_or_create_config();

    unsafe {
        state::APP_LAYOUT_RULES = Some(config);

        let _pool = NSAutoreleasePool::new(nil);

        let app = NSApp();
        app.setActivationPolicy_(NSApplicationActivationPolicyProhibited);

        core::observer::setup_observers();

        core::monitor::update_active_window();
        core::monitor::update_keyboard_layout();

        if let (Some(app), Some(layout)) = (
            &*std::ptr::addr_of!(state::CURRENT_APP),
            &*std::ptr::addr_of!(state::CURRENT_KEYBOARD_LAYOUT),
        ) {
            println!("Startup - Active window: {} | Layout: {}", app, layout);
        }

        println!("\nMonitoring started. Press Ctrl+C to exit.");
        println!("Config file: {}", core::config::get_config_path().display());

        core::macos_api::run_main_loop();
    }
}
