//! Software updates view.

use crate::app::App;
use crate::backend::updates::{self, Status};
use crate::components::dropdown::dropdown_option;
use crate::components::AppFn;
use crate::motion::{interactive, Motion, Spec};
use crate::theme;
use gpui::{prelude::*, div, px, rgb, AnyElement, ClickEvent, Context};
use std::rc::Rc;

impl App {
    pub fn render_updates_view(&mut self, cx: &mut Context<App>) -> AnyElement {
        let (branch, status) = {
            let state = self.state.read(cx);
            (state.config.branch.clone(), state.update_status.clone())
        };
        let supported = updates::is_supported();
        let busy = status.busy();
        let available = matches!(status, Status::Available { .. });
        let failed = matches!(status, Status::Error(_));
        let branch_items = vec![
            ("beta".to_string(), "beta".to_string()),
            ("main".to_string(), "main".to_string()),
        ];
        let locked = if busy || !supported {
            vec!["main".into(), "beta".into()]
        } else {
            vec![]
        };
        let on_branch_select = Rc::new(
            move |app: &mut App, cx: &mut Context<App>, val: String| app.set_branch(val, cx),
        );
        let description = update_description(self, &status);
        let description_alpha = self.motion.scalar(
            "updates-description", "opacity", 0., 1., Spec::motion(300),
        );
        let action_label = self.t(if available { "Update" } else { "Check" });
        let action = update_button(
            "update-action-btn", action_label, supported && !busy,
            Box::new(move |app, cx| {
                if available {
                    app.install_update(cx);
                } else {
                    app.check_updates(cx);
                }
            }), cx, &mut self.motion,
        );

        div()
            .id("updates-root")
            .flex()
            .flex_col()
            .gap(px(8.))
            .font_family("Poppins")
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(gpui::white())
            .child(dropdown_option(
                &self.t("Update branch"),
                &self.t("Changing the branch checks for updates; installation is manual."),
                (branch.clone(), branch),
                branch_items,
                locked,
                self.update_dropdown_open,
                Box::new(move |app, cx| {
                    if supported && !busy {
                        app.toggle_update_dropdown(cx);
                    }
                }),
                on_branch_select,
                cx,
                &mut self.motion,
            ))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(12.))
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
                            .child(div().text_size(px(14.)).child(self.t("Software update")))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(gpui::FontWeight::NORMAL)
                                    .text_color(rgb(if failed { theme::RED } else { 0xffffff }))
                                    .opacity(description_alpha)
                                    .child(description),
                            ),
                    )
                    .child(div().flex_shrink_0().child(action)),
            )
            .into_any_element()
    }
}

fn update_description(app: &App, status: &Status) -> String {
    match status {
        Status::Unsupported => app.t("Self-updating is supported on Windows x64 only."),
        Status::Idle => app.t("Check for a new version"),
        Status::Checking => app.t("Checking for updates…"),
        Status::NoRelease => app.t("No native Rust release is published for this branch yet."),
        Status::UpToDate => app.t("You are on the latest version!"),
        Status::Available { version } => format!("{} {version}", app.t("Update available:")),
        Status::Downloading { received, total } => {
            let percent = if *total == 0 { 0 } else { received.saturating_mul(100) / total };
            format!("{} {percent}%", app.t("Downloading update"))
        }
        Status::Verifying => app.t("Verifying download…"),
        Status::Preparing => app.t("Preparing update…"),
        Status::Restarting => app.t("Restarting as the current Windows user…"),
        Status::Error(error) => format!("{} {error}", app.t("Update failed:")),
    }
}

fn update_button(
    id: &'static str,
    label: String,
    enabled: bool,
    on_click: AppFn,
    cx: &mut Context<App>,
    motion: &mut Motion,
) -> AnyElement {
    let target = if !enabled {
        theme::INPUT_BG
    } else if motion.is_hovered(id) && !motion.is_pressed(id) {
        theme::ACCENT_HOVER
    } else {
        theme::ACCENT
    };
    let background = motion.color(
        id, "background", rgb(theme::ACCENT).into(), rgb(target).into(), Spec::css(100),
    );
    let alpha = motion.scalar(
        id, "opacity", 0., if enabled { 1. } else { 0.45 }, Spec::motion(300),
    );
    interactive(div().id(id), id.into(), cx)
        .text_size(px(12.))
        .py(px(8.))
        .px(px(16.))
        .bg(background)
        .opacity(alpha)
        .rounded(px(6.))
        .when(enabled, |this| this.cursor_pointer())
        .on_click(cx.listener(move |app, _: &ClickEvent, _, cx| {
            if enabled {
                on_click(app, cx);
            }
        }))
        .child(label)
        .into_any_element()
}
