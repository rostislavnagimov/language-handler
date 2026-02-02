use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicyProhibited};
use cocoa::base::nil;
use cocoa::foundation::NSAutoreleasePool;
use objc::{msg_send, sel, sel_impl};

mod monitor;
mod observer;
mod state;
mod switcher;

fn main() {
    let config = common::config::load_or_create_config();

    unsafe {
        state::APP_LAYOUT_RULES = Some(config);

        let _pool = NSAutoreleasePool::new(nil);
        let app = NSApp();
        
        // Запуск без GUI (headless)
        app.setActivationPolicy_(NSApplicationActivationPolicyProhibited);

        observer::setup_observers();
        monitor::update_active_window();
        monitor::update_keyboard_layout();

        if let (Some(app_name), Some(layout)) = (
            &*std::ptr::addr_of!(state::CURRENT_APP),
            &*std::ptr::addr_of!(state::CURRENT_KEYBOARD_LAYOUT),
        ) {
            println!("Startup - Active window: {} | Layout: {}", app_name, layout);
        }

        println!("\nDaemon started. Press Ctrl+C to exit.");
        println!("Config file: {}", common::paths::get_config_path().display());

        let _: () = msg_send![app, run];
    }
}