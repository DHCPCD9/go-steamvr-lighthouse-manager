//! Standalone install-or-portable chooser; no manager or Bluetooth state is created here.

use crate::backend::installer::{self, InstallPlan};
use crate::{icons, theme};
use gpui::{
    prelude::*, div, px, rgb, AnyElement, ClickEvent, Context, FontWeight, MouseButton,
    Render, SharedString, Window, WindowControlArea,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Choice {
    Install,
    Portable,
}

pub struct Installer {
    choice: Choice,
    plan: Option<InstallPlan>,
    destination: SharedString,
    busy: bool,
    error: Option<SharedString>,
}

impl Installer {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        let (plan, destination, error) = match installer::plan() {
            Ok(plan) => {
                let destination = plan.directory.display().to_string().into();
                (Some(plan), destination, None)
            }
            Err(error) => (
                None,
                "Install location unavailable".into(),
                Some(format!("Could not determine the install location: {error:#}").into()),
            ),
        };
        Self {
            choice: Choice::Install,
            plan,
            destination,
            busy: false,
            error,
        }
    }

    fn select(&mut self, choice: Choice, cx: &mut Context<Self>) {
        if !self.busy {
            self.choice = choice;
            cx.notify();
        }
    }

    fn portable(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.busy {
            crate::open_manager(cx);
            window.remove_window();
        }
    }

    fn install(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let plan = match self.plan.clone().map(Ok).unwrap_or_else(installer::plan) {
            Ok(plan) => plan,
            Err(error) => {
                self.error = Some(format!("Could not determine the install location: {error:#}").into());
                cx.notify();
                return;
            }
        };
        self.destination = plan.directory.display().to_string().into();
        self.plan = Some(plan.clone());
        self.busy = true;
        self.error = None;
        cx.notify();

        // Keep all filesystem work and process spawning off the UI thread. The
        // completion uses a weak entity, so an externally closed window is safe.
        let worker = cx.background_executor().spawn(async move {
            installer::install(&plan)
                .map_err(|error| format!("Installation failed: {error:#}"))?;
            installer::launch(&plan)
                .map_err(|error| format!("Installed, but could not launch: {error:#}"))
        });
        cx.spawn(async move |this, cx| {
            let result = worker.await;
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(()) => cx.quit(),
                    Err(error) => {
                        this.error = Some(error.into());
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    fn choice_card(&self, choice: Choice, cx: &mut Context<Self>) -> AnyElement {
        let selected = self.choice == choice;
        let install = choice == Choice::Install;
        div()
            .id(if install { "installer-install" } else { "installer-portable" })
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.))
            .min_h(px(206.))
            .gap(px(10.))
            .p(px(16.))
            .rounded(px(12.))
            .border_2()
            .border_color(rgb(if selected { theme::ACCENT } else { 0x353535 }))
            .bg(rgb(if selected { 0x1b2839 } else { theme::CARD }))
            .when(!self.busy, |this| {
                this.cursor_pointer().hover(|style| {
                    style.border_color(rgb(if selected { theme::ACCENT_HOVER } else { 0x666666 }))
                })
            })
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.select(choice, cx)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_color(rgb(if selected { theme::ACCENT_HOVER } else { theme::TEXT_DIM }))
                            .child(if install {
                                icons::download().into_any_element()
                            } else {
                                icons::base_station().w(px(20.)).h(px(24.)).into_any_element()
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(20.))
                            .rounded_full()
                            .border_2()
                            .border_color(rgb(if selected { theme::ACCENT } else { theme::TEXT_DIMMEST }))
                            .when(selected, |this| {
                                this.child(div().size(px(8.)).rounded_full().bg(rgb(theme::ACCENT)))
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(if install { "Install for this user" } else { "Use portable" }),
                    )
                    .when(install, |this| {
                        this.child(
                            div()
                                .text_size(px(10.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(theme::ACCENT_HOVER))
                                .child("RECOMMENDED · NO ADMIN NEEDED"),
                        )
                    })
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(18.))
                            .text_color(rgb(theme::TEXT_MUTED))
                            .child(if install {
                                "Keep the app in a permanent location for this user."
                            } else {
                                "Run this copy as it is. Nothing is installed or moved."
                            }),
                    ),
            )
            .when(install, |this| {
                this.child(
                    div()
                        .mt_auto()
                        .min_w(px(0.))
                        .text_size(px(11.))
                        .line_height(px(16.))
                        .text_color(rgb(theme::TEXT_DIM))
                        .child(div().child("INSTALL LOCATION"))
                        .child(
                            div()
                                .w_full()
                                .whitespace_normal()
                                .text_color(rgb(theme::TEXT_MUTED))
                                .child(self.destination.clone()),
                        ),
                )
            })
            .when(!install, |this| {
                this.child(
                    div()
                        .mt_auto()
                        .text_size(px(11.))
                        .text_color(rgb(theme::TEXT_DIM))
                        .child("You can install later with --install."),
                )
            })
            .into_any_element()
    }
}

impl Render for Installer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.busy;
        let install = self.choice == Choice::Install;
        let action = if busy {
            "Installing & launching…"
        } else if !install {
            "Launch portable"
        } else if self.error.is_some() {
            "Retry install & launch"
        } else {
            "Install & launch"
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(theme::BG))
            .text_color(gpui::white())
            .font_family("Poppins")
            .font_weight(FontWeight::NORMAL)
            .child(
                div()
                    .flex()
                    .items_center()
                    .flex_shrink_0()
                    .h(px(48.))
                    .border_b_1()
                    .border_color(rgb(0x2b2b2b))
                    .child(
                        div()
                            .id("installer-drag-region")
                            .flex()
                            .items_center()
                            .flex_1()
                            .h_full()
                            .pl(px(24.))
                            .text_size(px(12.))
                            .text_color(rgb(theme::TEXT_DIM))
                            .window_control_area(WindowControlArea::Drag)
                            .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
                            .child("SteamVR Lighthouse Manager"),
                    )
                    .child(
                        div()
                            .id("installer-close")
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(36.))
                            .mr(px(12.))
                            .rounded(px(6.))
                            .text_color(rgb(theme::TEXT_DIM))
                            .when(!busy, |this| {
                                this.cursor_pointer().hover(|style| style.bg(rgb(theme::CARD_HOVER)))
                            })
                            .when(busy, |this| this.opacity(0.35))
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                if !this.busy {
                                    cx.quit();
                                }
                            }))
                            .child(icons::close()),
                    ),
            )
            .child(
                div()
                    .id("installer-content")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .px(px(24.))
                    .py(px(16.))
                    .gap(px(16.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(16.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .flex_shrink_0()
                                    .size(px(56.))
                                    .rounded(px(14.))
                                    .bg(rgb(0x1b2839))
                                    .child(icons::grouped()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .child(
                                        div()
                                            .text_size(px(24.))
                                            .line_height(px(34.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child("Make yourself at home"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .line_height(px(18.))
                                            .text_color(rgb(theme::TEXT_MUTED))
                                            .child("Choose how you’d like to run Lighthouse Manager."),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(12.))
                            .child(self.choice_card(Choice::Install, cx))
                            .child(self.choice_card(Choice::Portable, cx)),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(18.))
                            .text_color(rgb(theme::TEXT_DIM))
                            .child("Your existing settings stay in their current config location in either mode."),
                    )
                    .when_some(self.error.clone(), |this, error| {
                        this.child(
                            div()
                                .rounded(px(8.))
                                .border_1()
                                .border_color(rgb(theme::RED))
                                .bg(theme::red_badge())
                                .p(px(12.))
                                .text_size(px(12.))
                                .line_height(px(18.))
                                .text_color(rgb(theme::RED))
                                .child(div().font_weight(FontWeight::MEDIUM).child(error))
                                .child(
                                    div()
                                        .mt(px(4.))
                                        .text_color(rgb(theme::TEXT_MUTED))
                                        .child("Retry installation, or launch portable to continue without installing."),
                                ),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .justify_between()
                    .gap(px(16.))
                    .px(px(24.))
                    .py(px(16.))
                    .border_t_1()
                    .border_color(rgb(0x2b2b2b))
                    .child(
                        div()
                            .id("installer-portable-direct")
                            .text_size(px(12.))
                            .text_color(rgb(theme::TEXT_MUTED))
                            .when(!busy, |this| {
                                this.cursor_pointer().hover(|style| style.text_color(gpui::white()))
                            })
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.portable(window, cx);
                            }))
                            .child(if busy { "Please keep this window open." } else { "Use portable instead" }),
                    )
                    .child(
                        div()
                            .id("installer-primary")
                            .flex_shrink_0()
                            .rounded(px(8.))
                            .px(px(20.))
                            .py(px(12.))
                            .text_size(px(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .bg(rgb(if busy { theme::ACCENT_DISABLED } else { theme::ACCENT }))
                            .when(!busy, |this| {
                                this.cursor_pointer().hover(|style| style.bg(rgb(theme::ACCENT_HOVER)))
                            })
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                if this.choice == Choice::Portable {
                                    this.portable(window, cx);
                                } else {
                                    this.install(cx);
                                }
                            }))
                            .child(action),
                    ),
            )
    }
}
