//! Routed views implemented as methods on the root `App`.

pub mod base_station_settings;
pub mod devices;
pub mod group_settings;
pub mod group_view;
pub mod software_settings;
pub mod updates;

use crate::app::App;
use crate::components::{AppFn, base_station};
use crate::models::{LighthouseStation, Route};
use crate::motion::{Motion, Spec, interactive};
use crate::theme;
use gpui::{prelude::*, AnyElement, ClickEvent, Context, SharedString, div, px, rgb};

#[derive(Default)]
pub struct ViewTransitions {
    stations: Vec<RetainedStation>,
    skip_mount_animation: bool,
}

struct RetainedStation {
    station: LighthouseStation,
    key: SharedString,
    index: usize,
    visible: bool,
    animate_entry: bool,
}

impl ViewTransitions {
    pub(super) fn for_navigation() -> Self {
        Self { skip_mount_animation: true, ..Self::default() }
    }

    fn sync_stations(&mut self, stations: Vec<LighthouseStation>) {
        for row in &mut self.stations {
            row.visible = false;
        }
        for (index, station) in stations.into_iter().enumerate() {
            if let Some(row) = self.stations.iter_mut().find(|row| row.station.id == station.id) {
                row.station = station;
                row.index = index;
                row.visible = true;
            } else {
                self.stations.push(RetainedStation {
                    key: format!("station-row:{}", station.id).into(),
                    station,
                    index,
                    visible: true,
                    animate_entry: !self.skip_mount_animation,
                });
            }
        }
        self.skip_mount_animation = false;
        if !self.stations.windows(2).all(|rows| rows[0].index <= rows[1].index) {
            self.stations.sort_unstable_by_key(|row| (row.index, !row.visible));
        }
    }
}

impl App {
    fn station_rows(
        &mut self,
        stations: Vec<LighthouseStation>,
        edit_mode: bool,
        cx: &mut Context<App>,
    ) -> Vec<AnyElement> {
        self.view_transitions.sync_stations(stations);
        let mut children = Vec::with_capacity(self.view_transitions.stations.len());
        self.view_transitions.stations.retain(|row| {
            let delay = if row.animate_entry { (row.index + 1) as f32 * 0.150 } else { 0. };
            let opacity = self.motion.presence_from(
                &row.key, row.visible, if row.animate_entry { 0. } else { 1. },
                if row.visible { Spec::motion(300).delay(delay) } else { Spec::motion(300) },
            );
            let x = self.motion.scalar(
                &row.key,
                "x",
                if row.animate_entry { -30. } else { 0. },
                if row.visible { 0. } else { -20. },
                if row.visible { Spec::transform().delay(delay) } else { Spec::transform() },
            );
            if !row.visible && opacity <= 0. && self.motion.settled(&row.key, "x") {
                return false;
            }
            let visible = row.visible;
            let station = &row.station;
            let sid = station.id.clone();
            let on_select: AppFn = Box::new(move |app, cx| {
                if visible { app.select_station(sid.clone(), cx); }
            });
            let pid = station.id.clone();
            let on_power: AppFn = Box::new(move |app, cx| {
                if visible { app.toggle_power(pid.clone(), cx); }
            });
            let iid = station.id.clone();
            let on_identify: AppFn = Box::new(move |app, cx| {
                if visible { app.identify(iid.clone(), cx); }
            });
            let oid = station.id.clone();
            let on_open: AppFn = Box::new(move |app, cx| {
                if visible { app.navigate(Route::BaseStationSettings(oid.clone()), cx); }
            });
            let card = base_station::base_station(
                station,
                edit_mode && self.selected.contains(&station.id),
                edit_mode,
                self.identify_cooldown.contains(&station.id),
                on_select,
                on_power,
                on_identify,
                on_open,
                cx,
                &mut self.motion,
            );
            children.push(
                div()
                    .id(row.key.clone())
                    .relative()
                    .left(px(x))
                    .flex_shrink_0()
                    .opacity(opacity)
                    .when(!visible, |this| {
                        this.capture_any_mouse_down(|_, _, cx| cx.stop_propagation())
                            .capture_any_mouse_up(|_, _, cx| cx.stop_propagation())
                    })
                    .child(card)
                    .into_any_element(),
            );
            true
        });
        children
    }
}

fn action_background(id: &str, resting: u32, pressed: u32, motion: &mut Motion) -> gpui::Hsla {
    let target = if motion.is_pressed(id) {
        pressed
    } else if motion.is_hovered(id) {
        theme::CARD_HOVER
    } else {
        resting
    };
    motion.color(id, "background", rgb(resting).into(), rgb(target).into(), Spec::css(200))
}

fn text_action(
    id: &'static str,
    label: String,
    on_click: AppFn,
    cx: &mut Context<App>,
    motion: &mut Motion,
) -> AnyElement {
    let background = action_background(id, theme::CARD, theme::DARKEST, motion);
    interactive(div().id(id), id.into(), cx)
        .bg(background)
        .text_color(rgb(theme::TEXT_MUTED))
        .font_family("Poppins")
        .w_full()
        .py(px(4.))
        .text_size(px(14.))
        .text_center()
        .rounded(px(6.))
        .cursor_pointer()
        .on_click(cx.listener(move |app, _: &ClickEvent, _w, cx| on_click(app, cx)))
        .child(label)
        .into_any_element()
}
