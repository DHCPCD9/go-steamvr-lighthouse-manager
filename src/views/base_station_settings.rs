//! Base station settings view.

use crate::app::App;
use crate::components::checkbox::checkbox_option;
use crate::components::dropdown::dropdown_option;
use crate::components::text_input::TextInput;
use crate::components::AppFn;
use crate::models::Route;
use crate::theme;
use gpui::{prelude::*, AnyElement, Context, Entity, div, px, rgb};
use std::rc::Rc;

/// Nickname / text input option wrapper.
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
    pub fn render_base_station_settings_view(&mut self, cx: &mut Context<App>) -> AnyElement {
        let id = match &self.route {
            Route::BaseStationSettings(id) => id.clone(),
            _ => {
                self.navigate(Route::Devices, cx);
                return div().into_any_element();
            }
        };

        let (station, other_channels) = {
            let s = self.state.read(cx);
            let station = s.lighthouses.get(&id).cloned();
            let other: Vec<String> = s
                .lighthouses
                .values()
                .filter(|bs| bs.id != id)
                .map(|bs| bs.channel.to_string())
                .collect();
            (station, other)
        };

        let Some(station) = station else {
            self.navigate(Route::Devices, cx);
            return div().into_any_element();
        };

        let nickname_input = self.nickname_input.clone();

        let channel_items: Vec<(String, String)> = (1..=16)
            .map(|i| (i.to_string(), i.to_string()))
            .collect();
        let channel_value = (station.channel.to_string(), station.channel.to_string());

        let set_channel_id = id.clone();
        let on_channel_select = Rc::new(move |app: &mut App, cx: &mut Context<App>, val: String| {
            if let Ok(channel) = val.parse::<u8>() {
                app.set_channel(set_channel_id.clone(), channel, cx);
            }
        });

        let forget_id = id.clone();
        let on_forget: AppFn = Box::new(move |app, cx| {
            app.forget_station(forget_id.clone(), cx);
        });

        let wake_id = id.clone();
        let on_wake: AppFn = Box::new(move |app, cx| {
            app.set_managed_flags(wake_id.clone(), 2, cx);
        });

        let off_id = id.clone();
        let on_off: AppFn = Box::new(move |app, cx| {
            app.set_managed_flags(off_id.clone(), 4, cx);
        });

        div()
            .id("bs-settings-root")
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .text_color(gpui::white())
                    .font_family("Poppins")
                    .child(input_option(
                        self.t("Nickname"),
                        self.t("Base station nickname to display"),
                        nickname_input,
                        cx,
                    )),
            )
            .child(
                div()
                    .text_color(gpui::white())
                    .font_family("Poppins")
                    .child(dropdown_option(
                        &self.t("Channel"),
                        &self.t("Base station channel to operate"),
                        channel_value,
                        channel_items,
                        other_channels,
                        self.channel_dropdown_open,
                        Box::new(move |app, cx| app.toggle_channel_dropdown(cx)),
                        on_channel_select,
                        cx,
                        &mut self.motion,
                    )),
            )
            .child(
                div()
                    .text_color(gpui::white())
                    .font_family("Poppins")
                    .child(checkbox_option(
                        (station.managed_flags & 2) > 0,
                        on_wake,
                        &self.t("Manage Power (Wake up)"),
                        &self.t("Awake this base station on SteamVR Launch"),
                        cx,
                        &mut self.motion,
                    )),
            )
            .child(
                div()
                    .text_color(gpui::white())
                    .font_family("Poppins")
                    .child(checkbox_option(
                        (station.managed_flags & 4) > 0,
                        on_off,
                        &self.t("Manage Power (Power off)"),
                        &self.t("Power off this base station on SteamVR Exit"),
                        cx,
                        &mut self.motion,
                    )),
            )
            .child(
                super::text_action(
                    "forget-station",
                    self.t("Forget this base station"),
                    on_forget,
                    cx,
                    &mut self.motion,
                ),
            )
            .into_any_element()
    }
}
