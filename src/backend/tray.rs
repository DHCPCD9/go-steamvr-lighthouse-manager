//! Windows tray lifetime and native visibility. All methods run on the GPUI thread.

use anyhow::{bail, Result};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{SetForegroundWindow, ShowWindow, SW_HIDE, SW_RESTORE},
};

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Show,
    Wake,
    Sleep,
    Quit,
}

pub struct Tray {
    icon: TrayIcon,
    hwnd: HWND,
    enabled: bool,
    hidden: bool,
    pub events: flume::Receiver<Action>,
}

impl Tray {
    pub fn new(window: &gpui::Window) -> Result<Self> {
        let RawWindowHandle::Win32(handle) = HasWindowHandle::window_handle(window)?.as_raw() else {
            bail!("Expected a Windows window handle");
        };
        let menu = Menu::new();
        let show = MenuItem::new("Show", true, None);
        let wake = MenuItem::new("Wake up", true, None);
        let sleep = MenuItem::new("Sleep", true, None);
        let quit = MenuItem::new("Quit", true, None);
        menu.append_items(&[
            &show,
            &PredefinedMenuItem::separator(),
            &wake,
            &sleep,
            &PredefinedMenuItem::separator(),
            &quit,
        ])?;
        let image = image::load_from_memory(include_bytes!("../../steamvr-lighthouse-manager.png"))?
            .resize_exact(32, 32, image::imageops::FilterType::Lanczos3)
            .into_rgba8();
        let icon = TrayIconBuilder::new()
            .with_id("lighthouse-manager")
            .with_tooltip("SteamVR Lighthouse Manager")
            .with_icon(Icon::from_rgba(image.into_raw(), 32, 32)?)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()?;

        let (tx, events) = flume::unbounded();
        let menu_tx = tx.clone();
        let (show, wake, sleep, quit) = (
            show.id().clone(), wake.id().clone(), sleep.id().clone(), quit.id().clone(),
        );
        // Install these once: toggling the setting reuses this controller.
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let action = if event.id == show {
                Action::Show
            } else if event.id == wake {
                Action::Wake
            } else if event.id == sleep {
                Action::Sleep
            } else if event.id == quit {
                Action::Quit
            } else {
                return;
            };
            let _ = menu_tx.send(action);
        }));
        TrayIconEvent::set_event_handler(Some(move |event| {
            if matches!(event,
                TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. }
                | TrayIconEvent::DoubleClick { button: MouseButton::Left, .. }
            ) {
                let _ = tx.send(Action::Show);
            }
        }));
        log::info!("Windows tray initialized");
        Ok(Self {
            icon,
            hwnd: HWND(handle.hwnd.get() as *mut std::ffi::c_void),
            enabled: true,
            hidden: false,
            events,
        })
    }

    pub fn set_enabled(&mut self, enabled: bool) -> Result<()> {
        if self.enabled != enabled {
            self.icon.set_visible(enabled)?;
            self.enabled = enabled;
        }
        if !enabled && self.hidden {
            self.show();
        }
        Ok(())
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn hide(&mut self) {
        if self.enabled && !self.hidden {
            unsafe { let _ = ShowWindow(self.hwnd, SW_HIDE); }
            self.hidden = true;
            log::info!("Window hidden to tray");
        }
    }

    pub fn show(&mut self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_RESTORE);
            let _ = SetForegroundWindow(self.hwnd);
        }
        self.hidden = false;
        log::info!("Window restored from tray");
    }
}
