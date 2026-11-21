//! Base station group card.

use crate::app::App;
use crate::components::base_station::icon_button;
use crate::components::AppFn;
use crate::icons;
use crate::models::LighthouseGroup;
use crate::motion::{interactive, Motion, Spec};
use crate::theme;
use gpui::{prelude::*, AnyElement, ClickEvent, Context, SharedString, div, px, rgb};

pub fn base_station_group(
    id: &str,
    group: &LighthouseGroup,
    channels_label: String,
    on_open: AppFn,
    on_power: AppFn,
    cx: &mut Context<App>,
    motion: &mut Motion,
) -> AnyElement {
    let prefix = SharedString::from(format!("group-{id}"));
    let open_id = SharedString::from(format!("{prefix}-open"));
    let power_id = SharedString::from(format!("{prefix}-power"));
    let settings_id = SharedString::from(format!("{prefix}-settings"));
    let on_open = std::rc::Rc::new(on_open);

    let card_id = SharedString::from(format!("{prefix}-card"));
    let background = motion.color(
        &card_id, "background", rgb(theme::CARD).into(),
        rgb(if motion.is_pressed(&card_id) { theme::CARD }
            else if motion.is_hovered(&card_id) { theme::CARD_HOVER }
            else { theme::CARD }).into(),
        Spec::css(200),
    );
    interactive(div().id(card_id.clone()), card_id, cx)
        .text_color(gpui::white())
        .flex()
        .flex_row()
        .flex_shrink_0()
        .justify_between()
        .bg(background)
        .rounded(px(2.))
        .p(px(16.))
        .items_center()
        .cursor_pointer()
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(16.))
                .items_center()
                .id(open_id.clone())
                .cursor_pointer()
                .on_click(cx.listener({
                    let on_open = on_open.clone();
                    move |app, _: &ClickEvent, _w, cx| {
                        (*on_open)(app, cx);
                    }
                }))
                .child(icons::grouped().text_color(rgb(theme::ICON_BLUE)))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .child(
                            div().flex().flex_row().gap(px(6.)).items_center().child(
                                group.name.clone(),
                            ),
                        )
                        .child(
                            div()
                                .text_color(rgb(theme::TEXT_MUTED))
                                .child(channels_label),
                        ),
                ),
        )
        .child(
            div().flex().flex_row().gap(px(8.)).child(icon_button(
                power_id.clone(),
                icons::power(),
                false,
                on_power,
                cx,
                motion,
            ))
            .child(icon_button(
                settings_id.clone(),
                icons::settings(),
                false,
                Box::new(move |app, cx| {
                    (*on_open)(app, cx);
                }),
                cx,
                motion,
            )),
        )
        .into_any_element()
}
