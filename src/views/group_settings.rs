//! Group settings view.

use crate::app::App;
use crate::components::checkbox::checkbox_option;
use crate::components::text_input::TextInput;
use crate::components::AppFn;
use crate::models::Route;
use crate::theme;
use gpui::{prelude::*, AnyElement, Context, Entity, div, px, rgb};

fn input_option(
    title: String,
    description: String,
    input: Entity<TextInput>,
    _cx: &mut Context<App>,
) -> AnyElement {
    div()
        .text_color(gpui::white())
        .font_family("Poppins")
        .child(
            div()
                .bg(rgb(theme::CARD))
                .min_h(px(70.))
                .py(px(12.))
                .px(px(8.))
                .rounded(px(6.))
                .flex()
                .flex_col()
                .justify_between()
                .items_center()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .justify_between()
                        .items_center()
                        .px(px(8.))
                        .w_full()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w(px(0.))
                                .pr(px(12.))
                                .gap(px(2.))
                                .child(div().text_size(px(16.)).child(title))
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .font_weight(gpui::FontWeight::NORMAL)
                                        .opacity(0.8)
                                        .child(description),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_shrink_0()
                                .items_center()
                                .child(
                                    div()
                                        .w(px(260.))
                                        .h(px(24.))
                                        .bg(rgb(theme::INPUT_BG))
                                        .rounded(px(6.))
                                        .px(px(12.))
                                        .flex()
                                        .items_center()
                                        .text_size(px(12.))
                                        .line_height(px(18.))
                                        .text_color(gpui::white())
                                        .child(input),
                                ),
                        ),
                ),
        )
        .into_any_element()
}

impl App {
    pub fn render_group_settings_view(&mut self, cx: &mut Context<App>) -> AnyElement {
        let gid = match &self.route {
            Route::GroupSettings(id) => id.clone(),
            _ => {
                self.navigate(Route::Devices, cx);
                return div().into_any_element();
            }
        };

        let managed_flags = self
            .state
            .read(cx)
            .config
            .groups
            .get(&gid)
            .map(|g| g.managed_flags)
            .unwrap_or(0);

        let nickname_input = self.nickname_input.clone();

        let wake_id = gid.clone();
        let on_wake: AppFn = Box::new(move |app, cx| {
            app.update_group_flags(wake_id.clone(), 2, cx);
        });

        let off_id = gid.clone();
        let on_off: AppFn = Box::new(move |app, cx| {
            app.update_group_flags(off_id.clone(), 4, cx);
        });

        let delete_id = gid.clone();
        let delete_label = self.t("Delete group");

        div()
            .id("group-settings-root")
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .text_color(gpui::white())
                    .font_family("Poppins")
                    .child(input_option(
                        self.t("Group name"),
                        self.t("Display group name"),
                        nickname_input,
                        cx,
                    )),
            )
            .child(
                div()
                    .text_color(gpui::white())
                    .font_family("Poppins")
                    .child(checkbox_option(
                        (managed_flags & 2) > 0,
                        on_wake,
                        &self.t("Manage Power (Power on)"),
                        &self.t("Wake up base stations on SteamVR Launch"),
                        cx,
                        &mut self.motion,
                    )),
            )
            .child(
                div()
                    .text_color(gpui::white())
                    .font_family("Poppins")
                    .child(checkbox_option(
                        (managed_flags & 4) > 0,
                        on_off,
                        &self.t("Manage Power (Power Off)"),
                        &self.t("Power off base stations on SteamVR Exit"),
                        cx,
                        &mut self.motion,
                    )),
            )
            .child(
                super::text_action(
                    "group-delete-btn",
                    delete_label,
                    Box::new(move |app, cx| app.delete_group(delete_id.clone(), cx)),
                    cx,
                    &mut self.motion,
                ),
            )
            .into_any_element()
    }
}
