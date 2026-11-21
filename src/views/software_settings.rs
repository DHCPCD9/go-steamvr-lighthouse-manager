//! Application settings view.

use crate::app::App;
use crate::components::checkbox::checkbox;
use crate::components::dropdown::dropdown_option;
use crate::components::AppFn;
use crate::i18n;
use crate::models::Route;
use crate::theme;
use crate::motion::{Motion, Spec, interactive};
use gpui::{prelude::*, AnyElement, ClickEvent, Context, div, px, rgb};
use std::rc::Rc;

impl App {
    pub fn render_software_settings_view(&mut self, cx: &mut Context<App>) -> AnyElement {
        let (is_steamvr_managed, is_steamvr_installed, allow_tray, version) = {
            let s = self.state.read(cx);
            (
                s.config.is_steamvr_managed,
                s.config.is_steamvr_installed,
                s.config.allow_tray,
                s.platform.version.clone(),
            )
        };

        let tray_available = self.tray_available();
        let language = self.language.clone();
        let language_items: Vec<(String, String)> = i18n::LANGUAGES
            .iter()
            .map(|(code, title)| (title.to_string(), code.to_string()))
            .collect();
        let language_value = (
            i18n::language_title(&language).to_string(),
            language.clone(),
        );

        let on_language_select =
            Rc::new(move |app: &mut App, cx: &mut Context<App>, val: String| {
                app.set_language(val, cx);
            });

        let on_power_manage: AppFn = Box::new(move |app, cx| {
            app.toggle_steamvr_management(cx);
        });
        let on_tray: AppFn = Box::new(move |app, cx| {
            app.toggle_tray(cx);
        });

        // "Manage Base Station Power" description varies.
        let power_description = if is_steamvr_installed {
            self.t("Manage Power based on SteamVR launched or not")
        } else {
            self.t("SteamVR not found or this platform is not supported")
        };
        let power_alpha = self.motion.scalar("settings-power-description", "opacity", 0., 1., Spec::motion(300));
        let tray_alpha = self.motion.scalar("settings-tray-description", "opacity", 0., 1., Spec::motion(300));
        let updates_alpha = self.motion.scalar("settings-updates-description", "opacity", 0., 1., Spec::motion(300));
        let updates_background = super::action_background(
            "settings-updates-link", theme::CARD, theme::CARD, &mut self.motion,
        );

        div()
            .id("software-settings-root")
            .flex()
            .flex_col()
            .gap(px(8.))
            .font_family("Poppins")
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(gpui::white())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .w_full()
                    .child(
                        div()
                            .text_color(gpui::white())
                            .font_family("Poppins")
                            .child(dropdown_option(
                                &self.t("Language"),
                                &self.t("Language of the interface"),
                                language_value,
                                language_items,
                                vec![],
                                self.language_dropdown_open,
                                Box::new(move |app, cx| app.toggle_language_dropdown(cx)),
                                on_language_select,
                                cx,
                                &mut self.motion,
                            )),
                    )
                    .child(
                        // Manage Base Station Power.
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .items_center()
                            .w_full()
                            .bg(rgb(theme::CARD))
                            .p(px(16.))
                            .rounded(px(6.))
                            .child(
                                div().flex().flex_col().flex_1().min_w(px(0.)).pr(px(12.)).child(
                                    div()
                                        .text_color(gpui::white())
                                        .text_size(px(14.))
                                        .child(self.t("Manage Base Station Power")),
                                )
                                .child(
                                    div()
                                        .text_color(gpui::white())
                                        .text_size(px(12.))
                                        .font_weight(gpui::FontWeight::NORMAL)
                                        .opacity(power_alpha)
                                        .child(power_description),
                                ),
                            )
                            .child(checkbox(
                                "settings-manage-power",
                                is_steamvr_managed,
                                !is_steamvr_installed,
                                on_power_manage,
                                cx,
                                &mut self.motion,
                            )),
                    )
                    .child(
                        // Allow tray.
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .items_center()
                            .w_full()
                            .bg(rgb(theme::CARD))
                            .p(px(16.))
                            .rounded(px(6.))
                            .child(
                                div().flex().flex_col().flex_1().min_w(px(0.)).pr(px(12.)).child(
                                    div()
                                        .text_color(gpui::white())
                                        .text_size(px(14.))
                                        .child(self.t("Allow tray")),
                                )
                                .child(
                                    div()
                                        .text_color(gpui::white())
                                        .text_size(px(12.))
                                        .font_weight(gpui::FontWeight::NORMAL)
                                        .opacity(tray_alpha)
                                        .child(self.t(if tray_available {
                                            "Run the application in the tray when SteamVR is active or when the main window is closed."
                                        } else {
                                            "Tray is unavailable on this platform or could not be initialized."
                                        })),
                                ),
                            )
                            .child(checkbox("settings-allow-tray", allow_tray, !tray_available, on_tray, cx, &mut self.motion)),
                    )
                    .child(
                        // Updates link.
                        interactive(
                            div().id("settings-updates-link"),
                            "settings-updates-link".into(),
                            cx,
                        )
                            .flex()
                            .flex_row()
                            .justify_between()
                            .items_center()
                            .w_full()
                            .bg(updates_background)
                            .cursor_pointer()
                            .p(px(16.))
                            .rounded(px(6.))
                            .on_click(cx.listener(|app, _: &ClickEvent, _w, cx| {
                                app.navigate(Route::Updates, cx);
                            }))
                            .child(
                                div().flex().flex_col().child(
                                    div()
                                        .text_color(gpui::white())
                                        .text_size(px(14.))
                                        .child(self.t("Updates")),
                                )
                                .child(
                                    div()
                                        .text_color(gpui::white())
                                        .text_size(px(12.))
                                        .font_weight(gpui::FontWeight::NORMAL)
                                        .opacity(updates_alpha)
                                        .child(self.t("Control application updates")),
                                ),
                            ),
                    ),
            )
            .child(
                // Footer.
                div()
                    .flex()
                    .flex_row()
                    .justify_between()
                    .pt(px(2.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_family("Poppins")
                            .font_weight(gpui::FontWeight::NORMAL)
                            .flex()
                            .flex_row()
                            .gap(px(12.))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap(px(4.))
                                    .child(self.t("Made By"))
                                    .child(link_text("settings-author", "Alumi", "https://lisek.cc", cx, &mut self.motion)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap(px(4.))
                                    .child(self.t("Design By"))
                                    .child(link_text("settings-designer", "Klonerovsky", "https://github.com/klonerovsky183", cx, &mut self.motion)),
                            )
                            .child(link_text("settings-source", &self.t("Source code"), "https://github.com/DHCPCD9/go-steamvr-lighthouse-manager", cx, &mut self.motion)),
                    )
                    .child(
                        div()
                            .font_family("Poppins")
                            .font_weight(gpui::FontWeight::NORMAL)
                            .text_size(px(12.))
                            .text_color(rgb(theme::TEXT_MUTED))
                            .child(format!("v{version}")),
                    ),
            )
            .into_any_element()
    }
}

/// A blue clickable link-style button.
fn link_text(id: &'static str, label: &str, url: &'static str, cx: &mut Context<App>, motion: &mut Motion) -> AnyElement {
    let target = if motion.is_hovered(id) { theme::ACCENT_HOVER } else { theme::ACCENT };
    let color = motion.color(id, "color", rgb(theme::ACCENT).into(), rgb(target).into(), Spec::css(150));
    interactive(div().id(id), id.into(), cx)
        .text_color(color)
        .cursor_pointer()
        .on_click(move |_, _, cx| cx.open_url(url))
        .child(label.to_string())
        .into_any_element()
}
