use cocoa::appkit::{NSApp, NSApplication};
use cocoa::base::{id, nil};
use cocoa::foundation::NSAutoreleasePool;
use objc::{class, msg_send, sel, sel_impl};

mod gui;

// Временно: reexport daemon модулей для совместимости
// В будущем GUI не будет содержать эту логику

fn main() {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let app = NSApp();
        
        app.setActivationPolicy_(
            cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular,
        );

        let _window = gui::create_and_show_window();

        let current_app: id = msg_send![class!(NSRunningApplication), currentApplication];
        let _: () = msg_send![current_app, activateWithOptions: 1 << 1];

        println!("GUI started.");
        println!("Config file: {}", common::paths::get_config_path().display());

        let _: () = msg_send![app, run];
    }
}