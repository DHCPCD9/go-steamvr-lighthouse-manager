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

            let bounds = Bounds::centered(None, size(px(700.), px(400.)), cx);

            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    // Frameless: no system title bar; the app draws its own.
                    titlebar: None,
                    window_decorations: Some(gpui::WindowDecorations::Client),
                    is_resizable: false,
                    ..Default::default()
                },
                |window, cx| cx.new(|cx| {
                    let mut app = App::new(cx);
                    #[cfg(target_os = "windows")]
                    app.initialize_tray(window, cx);
                    let entity = cx.weak_entity();
                    window.on_window_should_close(cx, move |_, cx| {
                        entity.update(cx, |app: &mut App, cx| app.quit(cx)).is_err()
                    });
                    app
                }),
            )
            .unwrap();

            cx.activate(true);
        });
}
