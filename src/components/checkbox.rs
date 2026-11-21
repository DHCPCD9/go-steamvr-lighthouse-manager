//! Animated toggle and labeled toggle option.

use crate::app::App;
use crate::components::AppFn;
use crate::motion::{interactive, Motion, Spec};
use crate::theme;
use gpui::{prelude::*, AnyElement, ClickEvent, Context, SharedString, div, px, rgb};

pub fn checkbox(
    id: &str,
    value: bool,
    disabled: bool,
    on_toggle: AppFn,
    cx: &mut Context<App>,
    motion: &mut Motion,
) -> AnyElement {
    let track_bg = motion.color(
        id, "background", rgb(theme::CARD_HOVER).into(),
        rgb(if value { theme::ACCENT } else { theme::CARD_HOVER }).into(),
        Spec::css(100),
    );
    let track_border = motion.color(
        id, "border", rgb(theme::TEXT_DIM).into(),
        rgb(if value { theme::ACCENT_BORDER } else { theme::TEXT_DIM }).into(),
        Spec::css(100),
    );
    let knob_bg = motion.color(
        id, "knob", rgb(theme::TEXT_MUTED).into(),
        rgb(if value { 0xffffff } else { theme::TEXT_MUTED }).into(),
        Spec::css(100),
    );
    let x = motion.scalar(id, "x", if value { 0. } else { 15. },
        if value { 18. } else { 0. }, Spec::spring(100));
    let opacity = motion.scalar(id, "opacity", if disabled { 0.5 } else { 1. },
        if disabled { 0.5 } else { 1. }, Spec::css(100));
    let toggle = div()
        .id(SharedString::from(id.to_owned()))
        .w(px(42.))
        .h(px(24.))
        .flex_shrink_0()
        .rounded(px(20.))
        .border_2()
        .border_color(track_border)
        .bg(track_bg)
        .flex()
        .items_center()
        .p(px(2.))
        .opacity(opacity)
        .child(
            div()
                .w(px(14.))
                .h(px(14.))
                .flex_shrink_0()
                .bg(knob_bg)
                .rounded(px(16.))
                .relative()
                .left(px(x)),
        );
    if disabled {
        toggle.into_any_element()
    } else {
        interactive(toggle, SharedString::from(id.to_owned()), cx)
            .cursor_pointer()
            .on_click(cx.listener(move |app, _: &ClickEvent, _w, cx| {
                cx.stop_propagation();
                on_toggle(app, cx);
            }))
            .into_any_element()
    }
}

pub fn checkbox_option(
    value: bool,
    on_toggle: AppFn,
    title: &str,
    description: &str,
    cx: &mut Context<App>,
    motion: &mut Motion,
) -> AnyElement {
    let id = format!("checkbox-option:{title}");
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
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w(px(0.))
                .pr(px(12.))
                .child(
                    div()
                        .text_color(gpui::white())
                        .text_size(px(14.))
                        .font_family("Poppins")
                        .child(title.to_string()),
                )
                .child(
                    div()
                        .text_color(gpui::white())
                        .opacity(0.8)
                        .text_size(px(12.))
                        .font_weight(gpui::FontWeight::NORMAL)
                        .font_family("Poppins")
                        .child(description.to_string()),
                ),
        )
        .child(checkbox(&id, value, false, on_toggle, cx, motion))
        .into_any_element()
}
