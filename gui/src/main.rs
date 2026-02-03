use cocoa::appkit::{NSApp, NSApplication};
use cocoa::base::{id, nil};
use cocoa::foundation::NSAutoreleasePool;
use objc::{class, msg_send, sel, sel_impl};

mod gui;
mod helpers;

// Временно: reexport daemon модулей для совместимости
// В будущем GUI не будет содержать эту логику

fn main() {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let app = NSApp();

        app.setActivationPolicy_(
            cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular,
        );

        println!("DEBUG: Calling create_and_show_window...");
        let _window = gui::create_and_show_window();
        println!("DEBUG: window created.");

        let current_app: id = msg_send![class!(NSRunningApplication), currentApplication];
        println!("DEBUG: activating app...");
        let _: () = msg_send![current_app, activateWithOptions: 1 << 1];

        // Set delegate to handle applicationShouldTerminateAfterLastWindowClosed:
        println!("DEBUG: registering/setting delegate...");
        let updater_class = gui::register_status_updater_class();
        let updater: id = msg_send![updater_class, new];
        let _: () = msg_send![app, setDelegate: updater];
        println!("DEBUG: delegate set.");

        println!("GUI started.");
        println!(
            "Config file: {}",
            common::paths::get_config_path().display()
        );

        let _: () = msg_send![app, run];
    }
}
