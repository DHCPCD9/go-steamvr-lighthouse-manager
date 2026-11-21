//! Breadcrumb navigation bar.

use crate::app::App;
use crate::models::Breadcrumb;
use crate::motion::Spec;
use crate::theme;
use gpui::{prelude::*, AnyElement, Bounds, ClickEvent, Context, Pixels, SharedString,
    canvas, div, px, rgb};
use std::cell::Cell;
use std::rc::Rc;

struct RetainedBreadcrumb {
    id: String,
    item: Breadcrumb,
    index: usize,
    active: bool,
    last: bool,
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}

/// Keeps exiting crumbs out of flex layout at their last measured positions.
#[derive(Default)]
pub struct BreadcrumbTransitions {
    target: Vec<Breadcrumb>,
    rows: Vec<RetainedBreadcrumb>,
    next_id: u64,
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}

pub fn render(app: &mut App, cx: &mut Context<App>) -> AnyElement {
    let transitions = &mut app.breadcrumb_transitions;
    if transitions.target != app.breadcrumbs {
        for row in &mut transitions.rows {
            row.active = false;
        }
        for (index, item) in app.breadcrumbs.iter().enumerate() {
            if let Some(row) = transitions.rows.iter_mut()
                .find(|row| !row.active && row.item == *item)
            {
                row.active = true;
                row.index = index;
                row.last = index + 1 == app.breadcrumbs.len();
            } else {
                let id = format!("chrome:breadcrumb:{}", transitions.next_id);
                transitions.next_id += 1;
                transitions.rows.push(RetainedBreadcrumb {
                    id,
                    item: item.clone(),
                    index,
                    active: true,
                    last: index + 1 == app.breadcrumbs.len(),
                    bounds: Rc::default(),
                });
            }
        }
        transitions.rows.sort_by_key(|row| (!row.active, row.index));
        transitions.target.clone_from(&app.breadcrumbs);
    }

    let root_bounds = transitions.bounds.get();
    let mut children = Vec::with_capacity(transitions.rows.len());
    transitions.rows.retain(|row| {
        let opacity = app.motion.presence(&row.id, row.active, Spec::motion(300).delay(0.1));
        let x = app.motion.scalar(&row.id, "x", -3.,
            if row.active { 0. } else { -3. }, Spec::transform().delay(0.1));
        let separator_id = format!("{}:separator", row.id);
        let separator_opacity = if row.index > 0 {
            app.motion.presence(&separator_id, row.active, Spec::motion(300).delay(0.1))
        } else { 0. };
        let separator_x = if row.index > 0 {
            app.motion.scalar(&separator_id, "x", -3.,
                if row.active { 0. } else { -3. }, Spec::transform().delay(0.1))
        } else { 0. };
        if !row.active && opacity <= 0. && separator_opacity <= 0. {
            return false;
        }
        let mut group = div()
            .id(SharedString::from(row.id.clone()))
            .relative()
            .flex()
            .flex_row()
            .flex_shrink_0()
            .items_center()
            .gap(px(6.));
        if !row.active {
            // popLayout: do not reserve width for outgoing text or guess glyph widths.
            let (Some(bounds), Some(root)) = (row.bounds.get(), root_bounds) else {
                return true;
            };
            group = group.absolute()
                .left(bounds.origin.x - root.origin.x)
                .top(bounds.origin.y - root.origin.y)
                .w(bounds.size.width)
                .h(bounds.size.height);
        } else {
            let captured = row.bounds.clone();
            group = group.child(
                canvas(move |bounds, _, _| captured.set(Some(bounds)), |_, _, _, _| {})
                    .absolute()
                    .top(px(0.))
                    .left(px(0.))
                    .size_full(),
            );
        }
        if row.index > 0 {
            group = group.child(
                div()
                    .relative()
                    .left(px(separator_x))
                    .opacity(separator_opacity)
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child(">"),
            );
        }
        let text = div()
            .id(SharedString::from(format!("{}:text", row.id)))
            .relative()
            .left(px(x))
            .opacity(opacity)
            .whitespace_nowrap()
            .text_color(rgb(if row.last { 0xffffff } else { theme::TEXT_MUTED }))
            .child(row.item.text.clone());
        let text = if row.active {
            let route = row.item.route.clone();
            text.cursor_pointer()
                .on_click(cx.listener(move |app, _: &ClickEvent, _w, cx| {
                    app.navigate(route.clone(), cx);
                }))
        } else {
            text
        };
        children.push(group.child(text).into_any_element());
        true
    });
    if children.is_empty() {
        return div().into_any_element();
    }
    let bounds = transitions.bounds.clone();
    div()
        .id("chrome:breadcrumbs")
        .relative()
        .flex()
        .flex_row()
        .text_size(px(24.))
        .text_color(gpui::white())
        .font_family("Poppins")
        .font_weight(gpui::FontWeight::MEDIUM)
        .px(px(24.))
        .items_center()
        .gap(px(6.))
        .h(px(36.))
        .child(
            canvas(move |rect, _, _| bounds.set(Some(rect)), |_, _, _, _| {})
                .absolute()
                .top(px(0.))
                .left(px(0.))
                .size_full(),
        )
        .children(children)
        .into_any_element()
}
