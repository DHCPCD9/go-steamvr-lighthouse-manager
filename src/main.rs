#![cfg_attr(all(target_os = "windows", not(test)), windows_subsystem = "windows")]

//! Application entry point: registers embedded assets and opens the GPUI window.

use gpui::{prelude::*, 
    App as GpuiApp, Application, Bounds, KeyBinding, WindowBounds, WindowOptions, px, size,
};

mod app;
mod assets;
mod backend;
mod components;
mod config;
mod i18n;
mod installer;
mod icons;
mod models;
mod state;
mod theme;
mod motion;
mod views;

use app::App;
use assets::Assets;
use components::text_input::{
    Backspace, Delete, End, Home, Left, Paste, Right, SelectAll, SelectLeft, SelectRight,
};

fn main() {
    #[cfg(target_os = "windows")]
    if let Some(result) = backend::update_apply::handle_startup() {
        if let Err(error) = result {
            eprintln!("Update helper failed: {error:#}");
            std::process::exit(1);
        }
        return;
    }

    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();

    Application::new()
        .with_assets(Assets)
        .run(|cx: &mut GpuiApp| {
            cx.text_system()
                .add_fonts(
                    assets::FONTS
                        .into_iter()
                        .map(std::borrow::Cow::Borrowed)
                        .collect(),
                )
                .expect("load embedded Poppins fonts");

            // Text-input key bindings.
            #[cfg(target_os = "macos")]
            let (select_all, paste) = ("cmd-a", "cmd-v");
            #[cfg(not(target_os = "macos"))]
            let (select_all, paste) = ("ctrl-a", "ctrl-v");

            cx.bind_keys([
                KeyBinding::new("backspace", Backspace, None),
                KeyBinding::new("delete", Delete, None),
                KeyBinding::new("left", Left, None),
                KeyBinding::new("right", Right, None),
                KeyBinding::new("shift-left", SelectLeft, None),
                KeyBinding::new("shift-right", SelectRight, None),
                KeyBinding::new(select_all, SelectAll, None),
                KeyBinding::new(paste, Paste, None),
                KeyBinding::new("home", Home, None),
                KeyBinding::new("end", End, None),
            ]);

            if backend::installer::should_offer() {
                let window_size = if backend::installer::is_update_only() {
                    size(px(460.), px(216.))
                } else {
                    size(px(480.), px(336.))
                };
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                            None, window_size, cx,
                        ))),
                        titlebar: None,
                        window_decorations: Some(gpui::WindowDecorations::Client),
                        #[cfg(not(target_os = "windows"))]
                        window_background: gpui::WindowBackgroundAppearance::Transparent,
                        is_resizable: false,
                        ..Default::default()
                    },
                    |window, cx| {
                        #[cfg(target_os = "windows")]
                        {
                            apply_window_icon(window);
                            round_window_corners(window);
                        }
                        let _ = window;
                        cx.new(installer::Installer::new)
                    },
                )
                .expect("open installer window");
                cx.activate(true);
            } else {
                open_manager(cx);
            }
        });
}

fn open_manager(cx: &mut GpuiApp) {
    let bounds = Bounds::centered(None, size(px(700.), px(400.)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            window_decorations: Some(gpui::WindowDecorations::Client),
            is_resizable: false,
            ..Default::default()
        },
        |window, cx| cx.new(|cx| {
            let mut app = App::new(cx);
            #[cfg(target_os = "windows")]
            {
                apply_window_icon(window);
                app.initialize_tray(window, cx);
            }
            let entity = cx.weak_entity();
            window.on_window_should_close(cx, move |_, cx| {
                entity.update(cx, |app: &mut App, cx| app.quit(cx)).is_err()
            });
            app
        }),
    )
    .expect("open manager window");
    cx.activate(true);
}

#[cfg(target_os = "windows")]
fn apply_window_icon(window: &gpui::Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::{HWND, LPARAM, WPARAM},
            System::LibraryLoader::GetModuleHandleW,
            UI::WindowsAndMessaging::{
                LoadImageW, SendMessageW, ICON_BIG, ICON_SMALL, IMAGE_ICON, LR_DEFAULTSIZE,
                LR_SHARED, WM_SETICON,
            },
        },
    };
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return;
    };
    let hwnd = HWND(win32.hwnd.get() as *mut _);
    let Ok(module) = (unsafe { GetModuleHandleW(None) }) else {
        return;
    };
    // winresource embeds the main icon at resource ID 1.
    let icon_id = PCWSTR(1 as *const u16);
    if let Ok(icon) =
        unsafe { LoadImageW(Some(module.into()), icon_id, IMAGE_ICON, 0, 0, LR_DEFAULTSIZE | LR_SHARED) }
    {
        unsafe {
            let _ = SendMessageW(hwnd, WM_SETICON, Some(WPARAM(ICON_BIG as usize)), Some(LPARAM(icon.0 as isize)));
            let _ = SendMessageW(hwnd, WM_SETICON, Some(WPARAM(ICON_SMALL as usize)), Some(LPARAM(icon.0 as isize)));
        }
    }
}

#[cfg(target_os = "windows")]
fn round_window_corners(window: &gpui::Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::{
        Foundation::HWND,
        Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
        },
    };
    if let Ok(handle) = HasWindowHandle::window_handle(window) {
        if let RawWindowHandle::Win32(win32) = handle.as_raw() {
            let hwnd = HWND(win32.hwnd.get() as *mut _);
            let preference = DWMWCP_ROUND;
            let _ = unsafe {
                DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_WINDOW_CORNER_PREFERENCE,
                    &preference as *const _ as *const _,
                    std::mem::size_of_val(&preference) as u32,
                )
            };
        }
    }
}
