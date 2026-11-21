//! Devices list view.

use crate::app::App;
use crate::components::base_station_group;
use crate::components::AppFn;
use crate::icons;
use crate::models::Route;
use crate::motion::{Motion, Spec, interactive};
use crate::theme;
use gpui::{
    prelude::*, AnyElement, ClickEvent, Context, IntoElement,
    Transformation, div, percentage, px, radians, rgb,
};

fn edit_rotation(degrees: f32) -> gpui::Radians {
    radians(degrees.to_radians())
}

fn icon_opacity(id: &str, disabled: bool, motion: &mut Motion) -> f32 {
    let target = if disabled {
        0.4
    } else if motion.is_pressed(id) {
        0.8
    } else if motion.is_hovered(id) {
        1.
    } else {
        0.8
    };
    motion.scalar(id, "icon-opacity", 0.8, target, Spec::css(200))
}

fn clickable_icon(
    id: &'static str,
    icon: impl IntoElement,
    active: bool,
    disabled: bool,
    enabled: bool,
    on_click: AppFn,
    cx: &mut Context<App>,
    motion: &mut Motion,
) -> AnyElement {
    let base = if active { theme::ACCENT } else { theme::CARD };
    let background = if disabled {
        motion.color(id, "background", rgb(base).into(), rgb(base).into(), Spec::css(200))
    } else {
        super::action_background(id, base, theme::CARD, motion)
    };
    let opacity = motion.scalar(
        id, "disabled-opacity", 1., if disabled { 0.5 } else { 1. }, Spec::css(200),
    );
    div()
        .id(id)
        .when(enabled && !disabled, |this| interactive(this, id.into(), cx))
        .flex()
        .flex_row()
        .justify_center()
        .items_center()
        .bg(background)
        .p(px(6.))
        .rounded(px(6.))
        .opacity(opacity)
        .when(enabled && !disabled, |this| {
            this.cursor_pointer().on_click(cx.listener(
                move |app, _: &ClickEvent, _w, cx| on_click(app, cx),
            ))
        })
        .child(icon)
        .into_any_element()
}

impl App {
    pub fn render_devices_view(&mut self, cx: &mut Context<App>) -> AnyElement {
        let (groups, stations, scanning, update_ready) = {
            let s = self.state.read(cx);
            (s.groups_sorted(), s.lighthouses_sorted(), s.scanning,
                matches!(s.update_status, crate::backend::updates::Status::Available { .. }))
        };
        let edit_mode = self.edit_mode;
        let selected_len = self.selected.len();
        let empty = stations.is_empty() && !scanning;
        let mut list_children: Vec<AnyElement> = Vec::new();

        // Group cards only have CSS interaction transitions; they are not staggered.
        for (gid, group) in &groups {
            let channels: Vec<String> = stations
                .iter()
                .filter(|bs| group.base_stations.contains(&bs.id))
                .map(|bs| bs.channel.to_string())
                .collect();
            let channels_label = if channels.is_empty() {
                String::new()
            } else {
                format!("{}: {}", self.t("Channels"), channels.join(", "))
            };
            let gid_nav = gid.clone();
            let on_open: AppFn = Box::new(move |app, cx| {
                app.navigate(Route::Group(gid_nav.clone()), cx);
            });
            let on_power: AppFn = Box::new(move |_app, _cx| {});
            list_children.push(base_station_group::base_station_group(
                gid, group, channels_label, on_open, on_power, cx, &mut self.motion,
            ));
        }
        list_children.extend(self.station_rows(stations, edit_mode, cx));

        let empty_opacity = self.motion.presence("devices-empty", empty, Spec::motion(300));
        if empty || empty_opacity > 0. {
            list_children.push(
                div()
                    .font_family("Poppins")
                    .text_center()
                    .text_size(px(14.))
                    .text_color(rgb(theme::TEXT_MUTED))
                    .opacity(empty_opacity)
                    .child(self.t(
                        "No base stations seem to be found, maybe other programs are connected to them?",
                    ))
                    .into_any_element(),
            );
        }

        let list = div()
            .id("devices-list")
            .flex()
            .flex_col()
            .gap(px(8.))
            .py(px(2.))
            .max_h(px(360.))
            .overflow_y_scroll()
            .children(list_children);
        let mut actions: Vec<AnyElement> = Vec::new();

        let update_visible = update_ready && !edit_mode;
        let update_alpha = self.motion.presence("action-update-presence", update_visible, Spec::motion(300));
        let tooltip_visible = update_visible && self.motion.is_hovered("action-update");
        let tooltip_alpha = self.motion.presence("action-update-tooltip", tooltip_visible, Spec::ease(300));
        if update_visible || update_alpha > 0. {
            let icon_alpha = icon_opacity("action-update", false, &mut self.motion);
            let tooltip = div()
                .absolute()
                .bottom(gpui::relative(1.25))
                .left(gpui::relative(0.5))
                .ml(px(-60.))
                .max_w(px(240.))
                .p(px(5.))
                .rounded(px(5.))
                .bg(gpui::black())
                .text_color(gpui::white())
                .font_family("Poppins")
                .font_weight(gpui::FontWeight(450.))
                .text_size(px(14.))
                .text_center()
                .whitespace_nowrap()
                .opacity(tooltip_alpha)
                .child(self.t("Update is ready!"));
            let button = clickable_icon(
                "action-update",
                icons::download().text_color(gpui::white()).opacity(icon_alpha),
                true,
                false,
                update_visible,
                Box::new(|app, cx| app.navigate(Route::Updates, cx)),
                cx,
                &mut self.motion,
            );
            actions.push(
                div()
                    .relative()
                    .opacity(update_alpha)
                    .child(button)
                    .when(tooltip_visible || tooltip_alpha > 0., |this| this.child(tooltip))
                    .into_any_element(),
            );
        }

        let refresh_visible = !edit_mode;
        let refresh_alpha = self.motion.presence("action-refresh-presence", refresh_visible, Spec::motion(300));
        if refresh_visible || refresh_alpha > 0. {
            let icon_alpha = icon_opacity("action-refresh", scanning, &mut self.motion);
            let icon = icons::refresh().text_color(gpui::white()).opacity(icon_alpha);
            let icon = if scanning {
                icon.with_transformation(Transformation::rotate(percentage(self.motion.spin("scan-spin", 1000))))
            } else {
                icon
            };
            actions.push(
                div()
                    .opacity(refresh_alpha)
                    .child(clickable_icon(
                        "action-refresh", icon, false, false, refresh_visible,
                        Box::new(|app, cx| app.start_scan(cx)), cx, &mut self.motion,
                    ))
                    .into_any_element(),
            );
        }

        let plus_alpha = icon_opacity("action-group", false, &mut self.motion);
        actions.push(clickable_icon(
            "action-group",
            icons::plus().text_color(gpui::white()).opacity(plus_alpha),
            false,
            selected_len < 1,
            true,
            Box::new(|app, cx| app.group_selected(cx)),
            cx,
            &mut self.motion,
        ));

        // popLayout: both icons share a fixed slot, with the outgoing icon
        // absolutely positioned so its exit never displaces the incoming icon.
        let pencil_alpha = self.motion.presence("edit-pencil", !edit_mode, Spec::linear(300));
        let off_alpha = self.motion.presence("edit-pencil-off", edit_mode, Spec::linear(300));
        let icon_alpha = icon_opacity("action-edit", false, &mut self.motion);
        let mut edit_icon = div().relative().w(px(24.)).h(px(24.));
        if !edit_mode || pencil_alpha > 0. {
            let angle = self.motion.scalar(
                "edit-pencil", "angle", 0., if edit_mode { -180. } else { 0. }, Spec::linear(300),
            );
            edit_icon = edit_icon.child(
                div().absolute().inset_0().opacity(pencil_alpha).child(
                    icons::pencil()
                        .text_color(gpui::white())
                        .opacity(icon_alpha)
                        .with_transformation(Transformation::rotate(edit_rotation(angle))),
                ),
            );
        }
        if edit_mode || off_alpha > 0. {
            let angle = self.motion.scalar("edit-pencil-off", "angle", -180., -365., Spec::linear(300));
            edit_icon = edit_icon.child(
                div().absolute().inset_0().opacity(off_alpha).child(
                    icons::pencil_off()
                        .text_color(gpui::white())
                        .opacity(icon_alpha)
                        .with_transformation(Transformation::rotate(edit_rotation(angle))),
                ),
            );
        }
        actions.push(clickable_icon(
            "action-edit", edit_icon, edit_mode, false, true,
            Box::new(|app, cx| app.toggle_edit_mode(cx)), cx, &mut self.motion,
        ));

        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .justify_between()
            .child(list)
            .child(div().flex().flex_row().justify_end().gap(px(8.)).children(actions))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::edit_rotation;
    use gpui::{point, px, TransformationMatrix};

    #[test]
    fn edit_icon_rotation_is_counterclockwise_and_can_pass_a_full_turn() {
        let point = TransformationMatrix::unit()
            .rotate(edit_rotation(-365.))
            .apply(point(px(1.), px(0.)));
        assert!((f32::from(point.x) - 0.996_194_7).abs() < 0.0001);
        assert!((f32::from(point.y) + 0.087_155_7).abs() < 0.0001);
    }
}
