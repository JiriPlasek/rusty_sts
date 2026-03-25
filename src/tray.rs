use std::sync::mpsc;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, MenuId};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub struct TrayMenuIds {
    pub open: MenuId,
    pub sync_now: MenuId,
    pub quit: MenuId,
}

pub struct TrayHandle {
    pub _icon: TrayIcon,
    pub ids: TrayMenuIds,
    pub menu_rx: mpsc::Receiver<MenuId>,
    pub click_rx: mpsc::Receiver<TrayIconEvent>,
}

fn find_window() -> Option<windows::Win32::Foundation::HWND> {
    use windows::core::w;
    use windows::Win32::UI::WindowsAndMessaging::*;
    unsafe { FindWindowW(None, w!("rusty-sts")).ok() }
}

/// Find our window by title and restore + focus it using Win32 API.
/// This works regardless of the eframe event loop state.
fn show_window_win32() {
    use windows::Win32::UI::WindowsAndMessaging::*;
    if let Some(hwnd) = find_window() {
        unsafe {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

/// Hide the window from the taskbar entirely using Win32 API.
pub fn hide_window_win32() {
    use windows::Win32::UI::WindowsAndMessaging::*;
    if let Some(hwnd) = find_window() {
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
}

pub fn create_tray() -> Result<TrayHandle, Box<dyn std::error::Error>> {
    let icon_bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(icon_bytes)?.into_rgba8();
    let (width, height) = img.dimensions();
    let icon = Icon::from_rgba(img.into_raw(), width, height)?;

    let menu = Menu::new();
    let open_item = MenuItem::new("Open", true, None);
    let sync_item = MenuItem::new("Sync Now", true, None);
    let quit_item = MenuItem::new("Quit", true, None);

    let ids = TrayMenuIds {
        open: open_item.id().clone(),
        sync_now: sync_item.id().clone(),
        quit: quit_item.id().clone(),
    };

    menu.append(&open_item)?;
    menu.append(&sync_item)?;
    menu.append(&quit_item)?;

    let tray_icon = TrayIconBuilder::new()
        .with_tooltip("rusty-sts")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()?;

    // Handle Open and Quit directly in callbacks via Win32 API.
    // This bypasses eframe's event loop which doesn't run reliably when minimized.
    let open_id = ids.open.clone();
    let quit_id = ids.quit.clone();
    let (menu_tx, menu_rx) = mpsc::channel();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if event.id == open_id {
            show_window_win32();
        } else if event.id == quit_id {
            std::process::exit(0);
        } else {
            // Forward other events (like Sync Now) to the channel
            let _ = menu_tx.send(event.id);
        }
    }));

    let (click_tx, click_rx) = mpsc::channel();
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let TrayIconEvent::Click {
            button: tray_icon::MouseButton::Left,
            button_state: tray_icon::MouseButtonState::Up,
            ..
        } = event
        {
            show_window_win32();
        } else {
            let _ = click_tx.send(event);
        }
    }));

    Ok(TrayHandle {
        _icon: tray_icon,
        ids,
        menu_rx,
        click_rx,
    })
}
