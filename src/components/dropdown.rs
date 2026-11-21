//! Dropdown option controlled by the root `App`.
//! The chevron rotates and options fade in over 150ms when opened.

use crate::app::App;
use crate::components::AppFn;
use crate::icons;
use crate::motion::{interactive, Motion, Spec};
use crate::theme;
use gpui::{prelude::*, AnyElement, ClickEvent, Context, SharedString, Transformation, div,
    percentage, px, rgb};
use std::rc::Rc;

pub fn dropdown_option(
    title: &str,
    description: &str,
    value: (String, String), // (display title, value)
    items: Vec<(String, String)>, // (display title, value)
    locked: Vec<String>,     // values occupied by other stations
    open: bool,
    on_open_toggle: AppFn,
    on_select: Rc<dyn Fn(&mut App, &mut Context<App>, String) + 'static>,
    cx: &mut Context<App>,
    motion: &mut Motion,
) -> AnyElement {
    let value_for_checks = value.1.clone();
    let id = format!("dropdown:{title}");
    let options_id = format!("{id}:options");
    let opacity = motion.presence(&options_id, open, Spec::motion(150));
    let angle = motion.scalar(&id, "angle", 0., if open { 0.5 } else { 0. }, Spec::css(150));

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
                                .child(
                                    div().text_size(px(16.)).child(title.to_string()),
                                )
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .font_weight(gpui::FontWeight::NORMAL)
                                        .opacity(0.8)
                                        .child(description.to_string()),
                                ),
                        )
                        .child(
                            div().flex().flex_col().flex_shrink_0().items_center().child(
                                div()
                                    .id(SharedString::from(format!("{id}:toggle")))
                                    .bg(rgb(theme::INPUT_BG))
                                    .min_w(px(52.))
                                    .h(px(24.))
                                    .rounded(px(6.))
                                    .pr(px(8.))
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .cursor_pointer()
                                    .on_click(cx.listener(
                                        move |this, _: &ClickEvent, _w, cx| {
                                            on_open_toggle(this, cx);
                                        },
                                    ))
                                    .child(
                                        div()
                                            .px(px(15.))
                                            .py(px(3.))
                                            .text_size(px(12.))
                                            .child(value.0.clone()),
                                    )
                                    .child(
                                        icons::chevron_down()
                                            .text_color(gpui::white())
                                            .with_transformation(
                                                Transformation::rotate(percentage(angle)),
                                            ),
                                    ),
                            ),
                        ),
                )
                .when(open || opacity > 0., |this| {
                    this.child(
                        div()
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .items_center()
                            .w_full()
                            .py(px(6.))
                            .gap(px(6.))
                            .opacity(opacity)
                            .children(items.iter().map(|(title, val)| {
                                let active = *val == value_for_checks;
                                let occupied = locked.contains(val) && *val != value_for_checks;
                                let can_select =
                                    *val != value_for_checks && !locked.contains(val);

                                let option_id = SharedString::from(format!("{id}:option:{val}"));
                                let target_bg = if active {
                                    rgb(theme::ACCENT).into()
                                } else if occupied {
                                    rgb(theme::INPUT_BG).into()
                                } else if can_select && motion.is_hovered(&option_id) {
                                    rgb(theme::CARD_HOVER).into()
                                } else {
                                    gpui::transparent_black()
                                };
                                let text_target = rgb(if occupied { theme::TEXT_DIM } else { 0xffffff }).into();
                                let background = motion.color(&option_id, "background",
                                    target_bg, target_bg, Spec::css(200));
                                let text_color = motion.color(&option_id, "text",
                                    text_target, text_target, Spec::css(200));
                                let option = div()
                                    .id(option_id.clone())
                                    .px(px(12.))
                                    .py(px(8.))
                                    .rounded(px(5.))
                                    .text_size(px(12.))
                                    .bg(background)
                                    .text_color(text_color)
                                    .child(title.clone());
                                if open && (can_select || active) {
                                    let val = val.clone();
                                    let on_select = on_select.clone();
                                    interactive(option, option_id, cx)
                                        .cursor_pointer()
                                        .on_click(cx.listener(
                                            move |app, _: &ClickEvent, _w, cx| {
                                                cx.stop_propagation();
                                                on_select(app, cx, val.clone());
                                            },
                                        ))
                                        .into_any_element()
                                } else {
                                    option.into_any_element()
                                }
                            })),
                    )
                }),
        )
        .into_any_element()
}
