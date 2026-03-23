//! Minimal test: verify global-hotkey works with CFRunLoop on macOS
//! Run with: cargo run --example test_hotkey_fix -p swictation-daemon

use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() {
    println!("=== Hotkey Test ===");
    println!("Creating GlobalHotKeyManager on main thread...");
    
    let manager = GlobalHotKeyManager::new().expect("Failed to create manager");
    let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyD);
    manager.register(hotkey).expect("Failed to register Ctrl+Shift+D");
    println!("Registered: Ctrl+Shift+D");
    
    let received = Arc::new(AtomicBool::new(false));
    let received_clone = received.clone();
    let toggle_id = hotkey.id();
    
    // Listener thread (same pattern as swictation hotkey.rs)
    std::thread::spawn(move || {
        loop {
            if let Ok(event) = GlobalHotKeyEvent::receiver().recv() {
                if event.id == toggle_id && event.state == HotKeyState::Pressed {
                    println!(">>> HOTKEY RECEIVED! Ctrl+Shift+D pressed <<<");
                    received_clone.store(true, Ordering::SeqCst);
                }
            }
        }
    });
    
    println!("Press Ctrl+Shift+D within 15 seconds...");
    println!("(Running CFRunLoop on main thread)");
    
    // Carbon's RegisterEventHotKey delivers via GetApplicationEventTarget(),
    // which requires the application event loop to be actively running.
    // Plain CFRunLoopRunInMode is NOT sufficient — we need RunApplicationEventLoop().
    #[cfg(target_os = "macos")]
    {
        #[link(name = "Carbon", kind = "framework")]
        extern "C" {
            fn RunApplicationEventLoop();
        }

        println!("Running Carbon application event loop on main thread...");
        println!("Press Ctrl+Shift+D to test. Ctrl+C to exit.");
        unsafe {
            RunApplicationEventLoop();
        }
    }
}
