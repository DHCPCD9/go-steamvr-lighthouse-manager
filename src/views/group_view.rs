//! Station list within a group.

use crate::app::App;
use crate::models::Route;
use gpui::{prelude::*, AnyElement, Context, div, px};

impl App {
    pub fn render_group_view(&mut self, cx: &mut Context<App>) -> AnyElement {
        let gid = match &self.route {
            Route::Group(id) => id.clone(),
            _ => {
                self.navigate(Route::Devices, cx);
                return div().into_any_element();
            }
        };

        let stations = {
            let s = self.state.read(cx);
            let ids = s.config.groups.get(&gid).map(|g| g.base_stations.as_slice()).unwrap_or_default();
            s.lighthouses_sorted()
                .into_iter()
                .filter(|bs| ids.contains(&bs.id))
                .collect()
        };
        let list_children = self.station_rows(stations, false, cx);

        let edit_group_id = gid.clone();
        let edit_label = self.t("Edit group");

        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .justify_between()
            .child(
                div()
                    .id("group-list")
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .py(px(2.))
                    .max_h(px(360.))
                    .overflow_y_scroll()
                    .children(list_children),
            )
            .child(
                super::text_action(
                    "group-edit-btn",
                    edit_label,
                    Box::new(move |app, cx| {
                        app.navigate(Route::GroupSettings(edit_group_id.clone()), cx);
                    }),
                    cx,
                    &mut self.motion,
                ),
            )
            .into_any_element()
    }
}
