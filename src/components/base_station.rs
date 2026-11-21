//! Base station card and controls.

use crate::app::App;
use crate::components::AppFn;
use crate::icons;
use crate::models::LighthouseStation;
use crate::motion::{interactive, Motion, Spec};
use crate::theme;
use gpui::{prelude::*, AnyElement, ClickEvent, Context, SharedString, div, px, rgb};

/// Target status-dot color: sleep/error=red, preloaded=blue, awake=green.
pub fn dot_color(status: &str, is_awake: bool) -> gpui::Hsla {
    if status == "error" {
        return rgb(theme::RED).into();
    }
    if status != "ready" {
        return rgb(theme::CONNECTING).into();
    }
    if is_awake {
        return rgb(theme::GREEN).into();
    }
    rgb(theme::RED).into()
}

/// A card icon with an interruptible 150ms hover/disabled fade.
pub fn icon_button(
    id: SharedString,
    icon: gpui::Svg,
    disabled: bool,
    on_click: AppFn,
    cx: &mut Context<App>,
    motion: &mut Motion,
) -> AnyElement {
    icon_button_inner(id, icon, disabled, !disabled, on_click, cx, motion)
}

fn icon_button_inner(
    id: SharedString,
    icon: gpui::Svg,
    disabled: bool,
    interactive_now: bool,
    on_click: AppFn,
    cx: &mut Context<App>,
    motion: &mut Motion,
) -> AnyElement {
    let target = if disabled { 0.25 } else if motion.is_hovered(&id) { 1. } else { 0.75 };
    let opacity = motion.scalar(&id, "opacity", if disabled { 0.25 } else { 0.75 },
        target, Spec::css(150));
    let button = div()
        .id(id.clone())
        .p(px(4.))
        .rounded(px(6.))
        .opacity(opacity)
        .child(icon.text_color(rgb(theme::TEXT_MUTED)));
    if interactive_now {
        interactive(button, id, cx)
            .cursor_pointer()
            .on_click(cx.listener(move |app, _: &ClickEvent, _w, cx| {
                cx.stop_propagation();
                on_click(app, cx);
            }))
            .into_any_element()
    } else {
        button
            .on_click(cx.listener(|_app, _: &ClickEvent, _w, cx| {
                cx.stop_propagation();
            }))
            .into_any_element()
    }
}

pub fn base_station(
    station: &LighthouseStation,
    selected: bool,
    edit_mode: bool,
    identify_disabled: bool,
    on_select: AppFn,
    on_power: AppFn,
    on_identify: AppFn,
    on_open: AppFn,
    cx: &mut Context<App>,
    motion: &mut Motion,
) -> AnyElement {
    let is_awake = station.is_awake();
    let ready = station.status == "ready";
    let card_id = SharedString::from(format!("bs-card-{}", station.id));
    let background = motion.color(
        &card_id, "background", rgb(theme::CARD).into(),
        rgb(if motion.is_pressed(&card_id) {
            theme::CARD
        } else if selected || motion.is_hovered(&card_id) {
            theme::CARD_HOVER
        } else {
            theme::CARD
        }).into(),
        Spec::css(200),
    );
    let dot_target = dot_color(&station.status, is_awake);
    let dot = motion.color(&card_id, "status", dot_target, dot_target, Spec::css(300));
    let subtitle_id = format!("bs-subtitle-{}", station.id);
    let subtitle = motion.swap(&subtitle_id, ready, Spec::motion(300));

    let card = interactive(
        div()
            .id(card_id.clone())
            .text_color(gpui::white())
            .flex()
            .flex_row()
            .justify_between()
            .bg(background)
            .rounded(px(2.))
            .p(px(16.))
            .items_center()
            .cursor_pointer(),
        card_id, cx,
    );
    let card = if edit_mode {
        card.on_click(cx.listener(move |app, _: &ClickEvent, _w, cx| {
            on_select(app, cx);
        }))
    } else {
        card
    };

    let mut buttons = Vec::with_capacity(3);
    let controls = [
        (format!("bs-identify-{}", station.id), icons::eye(),
            is_awake && ready && !edit_mode, identify_disabled, on_identify),
        (format!("bs-power-{}", station.id), icons::power(),
            ready && !edit_mode, false, on_power),
        (format!("bs-open-{}", station.id), icons::chevron_right(),
            !edit_mode, false, on_open),
    ];
    for (id, icon, visible, disabled, callback) in controls {
        let alpha = motion.presence(&id, visible, Spec::motion(300));
        if visible || alpha > 0. {
            buttons.push(
                div().opacity(alpha).child(icon_button_inner(
                    SharedString::from(id), icon, disabled, visible && !disabled,
                    callback, cx, motion,
                )).into_any_element(),
            );
        }
    }

    card.child(
        div()
            .flex()
            .flex_row()
            .gap(px(16.))
            .items_center()
            .child(icons::base_station().text_color(rgb(theme::ICON_BLUE)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(6.))
                            .items_center()
                            .child(station.name.clone())
                            .child(icons::status_circle().text_color(dot)),
                    )
                    .child(
                        div()
                            .text_color(rgb(theme::TEXT_MUTED))
                            .opacity(subtitle.opacity)
                            .child(if subtitle.value {
                                format!("Channel {}", station.channel)
                            } else {
                                "Connecting...".to_string()
                            }),
                    ),
            ),
    )
    .child(div().flex().flex_row().gap(px(8.)).children(buttons))
    .into_any_element()
}
