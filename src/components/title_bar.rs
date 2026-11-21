//! Application title bar and window controls.

use crate::app::App;
use crate::icons;
use crate::motion::Spec;
use crate::theme;
use gpui::{prelude::*, AnyElement, ClickEvent, Context, div, px, rgb};

pub fn render(app: &mut App, cx: &mut Context<App>) -> AnyElement {
    let (show_steamvr, steamvr_active, steamvr_known, backend_ready, version) = {
        let s = app.state.read(cx);
        (
            s.config.is_steamvr_installed && s.config.is_steamvr_managed,
            s.steamvr_active,
            s.steamvr_state_known,
            s.backend_ready,
            s.platform.version.clone(),
        )
    };
    let active_label = app.t(if !steamvr_known {
        "Checking"
    } else if steamvr_active {
        "Active"
    } else {
        "Waiting"
    });
    let status_target = rgb(if !steamvr_known {
        theme::TEXT_MUTED
    } else if steamvr_active {
        theme::GREEN
    } else {
        theme::RED
    }).into();
    let status_color = app.motion.color("chrome:steamvr", "status",
        status_target, status_target, Spec::css(200));
    let steamvr_alpha = app.motion.presence("chrome:steamvr", show_steamvr, Spec::motion(300));
    let error_alpha = app.motion.presence("chrome:backend-error", !backend_ready, Spec::motion(300));

    div()
        .flex()
        .flex_row()
        .justify_between()
        .pt(px(16.))
        .px(px(24.))
        .child(
            div()
                .flex()
                .flex_row()
                .items_baseline()
                .gap(px(4.))
                .child(
                    div()
                        .text_color(rgb(theme::TEXT_DIM))
                        .font_family("Poppins")
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .child("SteamVR Lighthouse Manager"),
                )
                .child(
                    div()
                        .text_color(rgb(theme::TEXT_DIMMEST))
                        .font_family("Poppins")
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_size(px(12.))
                        .line_height(px(20.))
                        .child(version),
                ),
        )
        .child(div())
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(4.))
                .items_center()
                .when(show_steamvr || steamvr_alpha > 0., |this| {
                    this.child(
                        div()
                            .opacity(steamvr_alpha)
                            .flex()
                            .flex_row()
                            .gap(px(4.))
                            .text_color(gpui::white())
                            .font_family("Poppins")
                            .text_size(px(14.))
                            .items_center()
                            .px(px(8.))
                            .child(
                                div()
                                    .text_color(rgb(theme::TEXT_MUTED))
                                    .child("SteamVR"),
                            )
                            .child(div().text_color(status_color).child(active_label)),
                    )
                })
                .when(!backend_ready || error_alpha > 0., |this| {
                    this.child(
                        div()
                            .opacity(error_alpha)
                            .px(px(8.))
                            .font_family("Poppins")
                            .text_size(px(14.))
                            .child(
                                div()
                                    .text_color(rgb(theme::RED))
                                    .bg(gpui::rgba(0xff737333))
                                    .rounded(px(12.))
                                    .border_color(rgb(theme::RED))
                                    .border(px(0.5))
                                    .p(px(4.))
                                    .child("Bluetooth connection failed."),
                            ),
                    )
                })
                .child(
                    crate::components::base_station::icon_button(
                        gpui::SharedString::from("chrome:titlebar-power"),
                        icons::power(),
                        false,
                        Box::new(|app, cx| app.toggle_all_power(cx)),
                        cx,
                        &mut app.motion,
                    ),
                )
                .child(
                    div()
                        .id("chrome:titlebar-settings")
                        .cursor_pointer()
                        .on_click(cx.listener(|app, _: &ClickEvent, _w, cx| {
                            app.navigate(crate::models::Route::SoftwareSettings, cx);
                        }))
                        .child(icons::settings().text_color(rgb(theme::TEXT_DIM))),
                )
                .child(
                    div()
                        .id("chrome:titlebar-close")
                        .cursor_pointer()
                        .on_click(cx.listener(|app, _: &ClickEvent, _w, cx| {
                            app.quit(cx);
                        }))
                        .child(icons::close().text_color(rgb(theme::TEXT_DIM))),
                ),
        )
        .into_any_element()
}
