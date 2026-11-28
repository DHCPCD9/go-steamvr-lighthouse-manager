//! Standalone install-or-portable chooser; no manager or Bluetooth state is created here.

use crate::backend::installer::{self, InstallPlan, InstallProgress};
use crate::motion::{Motion, Spec};
use crate::{icons, theme};
use gpui::{
    prelude::*, div, px, relative, rgb, AnyElement, ClickEvent, Context, FontWeight, MouseButton,
    Render, SharedString, Window, WindowControlArea,
};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Choice {
    Install,
    Portable,
}

pub struct Installer {
    update_only: bool,
    choice: Choice,
    plan: Option<InstallPlan>,
    destination: SharedString,
    busy: bool,
    progress: f32,
    stage: SharedString,
    error: Option<SharedString>,
    motion: Motion,
    animation_frame_pending: bool,
}

impl Installer {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let update_only = installer::is_update_only();
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
        let mut this = Self {
            update_only,
            choice: Choice::Install,
            plan,
            destination,
            busy: false,
            progress: 0.0,
            stage: if update_only {
                "Starting update…".into()
            } else {
                "Preparing installation…".into()
            },
            error,
            motion: Motion::default(),
            animation_frame_pending: false,
        };
        if update_only && this.error.is_none() {
            this.start_install(true, cx);
        }
        this
    }

    fn select(&mut self, choice: Choice, cx: &mut Context<Self>) {
        if !self.busy && self.choice != choice {
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
        self.start_install(false, cx);
    }

    fn start_install(&mut self, migrate_legacy: bool, cx: &mut Context<Self>) {
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
        self.progress = 0.04;
        self.stage = if migrate_legacy {
            "Waiting for old version to close…".into()
        } else {
            "Preparing install folder…".into()
        };
        self.error = None;
        cx.notify();

        let (tx, rx) = flume::unbounded::<InstallProgress>();
        let ui_this = cx.weak_entity();
        cx.spawn(async move |_, cx| {
            while let Ok(update) = rx.recv_async().await {
                if ui_this
                    .update(cx, |this, cx| {
                        this.progress = update.fraction.clamp(0.0, 1.0);
                        this.stage = update.stage.into();
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();

        let update_only = self.update_only;
        // Keep all filesystem work and process spawning off the UI thread. The
        // completion uses a weak entity, so an externally closed window is safe.
        let worker = cx.background_executor().spawn(async move {
            let result = if update_only {
                installer::migrate_go_and_install(&plan, migrate_legacy, |progress| {
                    let _ = tx.send(progress);
                })
            } else {
                installer::install_with_progress(&plan, |progress| {
                    let _ = tx.send(progress);
                })
            };
            result.map_err(|error| {
                if update_only {
                    format!("Update failed: {error:#}")
                } else {
                    format!("Installation failed: {error:#}")
                }
            })?;
            let _ = tx.send(InstallProgress {
                fraction: 0.96,
                stage: "Launching application…",
            });
            installer::launch(&plan)
                .map_err(|error| format!("Installed, but could not launch: {error:#}"))?;
            let _ = tx.send(InstallProgress {
                fraction: 1.0,
                stage: "Launching application…",
            });
            Ok::<(), String>(())
        });
        cx.spawn(async move |this, cx| {
            let result = worker.await;
            match result {
                Ok(()) => {
                    let _ = this.update(cx, |this, cx| {
                        this.progress = 1.0;
                        this.stage = "Launching application…".into();
                        cx.notify();
                    });
                    // Allow the progress bar to visually complete before closing.
                    cx.background_executor().timer(Duration::from_millis(220)).await;
                    let _ = this.update(cx, |_, cx| cx.quit());
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.busy = false;
                        this.progress = 0.0;
                        this.error = Some(error.into());
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    fn choice_row(&self, choice: Choice, cx: &mut Context<Self>) -> AnyElement {
        let selected = self.choice == choice;
        let install = choice == Choice::Install;
        let busy = self.busy;

        let install_desc = if cfg!(target_os = "windows") {
            "Adds a Start Menu shortcut. No admin privileges needed."
        } else if cfg!(target_os = "macos") {
            "Installs to ~/Applications. No admin privileges needed."
        } else {
            "Adds an app menu launcher. No admin privileges needed."
        };

        div()
            .id(if install { "installer-install" } else { "installer-portable" })
            .flex()
            .flex_col()
            .gap(px(8.))
            .w_full()
            .px(px(14.))
            .py(px(11.))
            .rounded(px(10.))
            .border_1()
            .border_color(if selected {
                theme::c_a(theme::ACCENT, 0.85)
            } else {
                theme::c(0x2b2b2b)
            })
            .bg(if selected {
                rgb(0x1a2433)
            } else {
                rgb(theme::CARD)
            })
            .when(busy, |this| this.opacity(0.55))
            .when(!busy, |this| {
                this.cursor_pointer().hover(|style| {
                    style
                        .border_color(if selected {
                            theme::c(theme::ACCENT_HOVER)
                        } else {
                            theme::c(0x454545)
                        })
                        .bg(if selected {
                            rgb(0x1e2a3c)
                        } else {
                            rgb(0x242424)
                        })
                })
            })
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.select(choice, cx)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .size(px(34.))
                            .rounded(px(8.))
                            .bg(if selected {
                                theme::c_a(theme::ACCENT, 0.16)
                            } else {
                                theme::c(0x282828)
                            })
                            .child(if install {
                                icons::download()
                                    .w(px(18.))
                                    .h(px(18.))
                                    .text_color(rgb(if selected {
                                        theme::ACCENT_HOVER
                                    } else {
                                        theme::TEXT_MUTED
                                    }))
                                    .into_any_element()
                            } else {
                                icons::base_station()
                                    .w(px(14.))
                                    .h(px(18.))
                                    .text_color(rgb(if selected {
                                        theme::ACCENT_HOVER
                                    } else {
                                        theme::TEXT_MUTED
                                    }))
                                    .into_any_element()
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w(px(0.))
                            .gap(px(2.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(7.))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .line_height(px(18.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(if install {
                                                "Install for this user"
                                            } else {
                                                "Use portable"
                                            }),
                                    )
                                    .when(install, |this| {
                                        this.child(
                                            div()
                                                .px(px(6.))
                                                .py(px(1.))
                                                .rounded_full()
                                                .bg(theme::c_a(theme::ACCENT, 0.16))
                                                .text_size(px(9.5))
                                                .line_height(px(14.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(rgb(theme::ACCENT_HOVER))
                                                .child("Recommended"),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .line_height(px(16.))
                                    .text_color(rgb(theme::TEXT_DIM))
                                    .child(if install {
                                        install_desc
                                    } else {
                                        "Run directly from here without copying files or shortcuts."
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .size(px(16.))
                            .rounded_full()
                            .border_1()
                            .border_color(rgb(if selected {
                                theme::ACCENT
                            } else {
                                theme::TEXT_DIMMEST
                            }))
                            .when(selected, |this| {
                                this.bg(rgb(theme::ACCENT)).child(
                                    div()
                                        .size(px(6.))
                                        .rounded_full()
                                        .bg(gpui::white()),
                                )
                            }),
                    ),
            )
            .when(install, |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .w_full()
                        .min_w(px(0.))
                        .px(px(8.))
                        .py(px(5.))
                        .rounded(px(6.))
                        .bg(rgb(theme::INPUT_BG))
                        .border_1()
                        .border_color(if selected {
                            theme::c_a(theme::ACCENT, 0.22)
                        } else {
                            theme::c(0x262626)
                        })
                        .text_size(px(10.5))
                        .line_height(px(14.))
                        .child(
                            div()
                                .flex_shrink_0()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(theme::TEXT_DIM))
                                .child("Path"),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.))
                                .truncate()
                                .text_color(rgb(theme::TEXT_MUTED))
                                .child(self.destination.clone()),
                        ),
                )
            })
            .into_any_element()
    }

    fn render_progress_bar(&mut self) -> AnyElement {
        let filled = self
            .motion
            .scalar(
                "installer:progress",
                "fill",
                0.0,
                self.progress.clamp(0.0, 1.0),
                Spec::css(180),
            )
            .clamp(0.0, 1.0);
        let shimmer = self.motion.spin("installer:shimmer", 1300);
        let percent = (filled * 100.0).round() as u32;

        div()
            .flex()
            .flex_col()
            .w_full()
            .gap(px(7.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .truncate()
                            .text_size(px(11.5))
                            .line_height(px(16.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(theme::TEXT_MUTED))
                            .child(self.stage.clone()),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(px(11.5))
                            .line_height(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(theme::ACCENT_HOVER))
                            .child(format!("{percent}%")),
                    ),
            )
            .child(
                div()
                    .relative()
                    .w_full()
                    .h(px(6.))
                    .rounded_full()
                    .bg(rgb(0x282828))
                    .overflow_hidden()
                    .child(
                        div()
                            .relative()
                            .h_full()
                            .w(relative(filled.max(0.03)))
                            .rounded_full()
                            .bg(rgb(theme::ACCENT))
                            .overflow_hidden()
                            .child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .bottom_0()
                                    .left(relative(-0.35 + shimmer * 1.45))
                                    .w(relative(0.35))
                                    .rounded_full()
                                    .bg(theme::c_a(theme::ACCENT_BORDER, 0.42)),
                            ),
                    ),
            )
            .into_any_element()
    }
}

impl Render for Installer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.motion.begin_frame(Instant::now());

        let busy = self.busy;
        let update_only = self.update_only;
        let install = self.choice == Choice::Install;
        let version = env!("CARGO_PKG_VERSION").split('-').next().unwrap_or(env!("CARGO_PKG_VERSION"));
        let action = if update_only {
            "Retry update"
        } else if !install {
            "Launch portable"
        } else if self.error.is_some() {
            "Retry install"
        } else {
            "Install & launch"
        };
        let footer = if busy {
            self.render_progress_bar()
        } else if update_only {
            div()
                .flex()
                .items_center()
                .w_full()
                .gap(px(8.))
                .child(
                    div()
                        .id("installer-skip-legacy")
                        .flex()
                        .items_center()
                        .justify_center()
                        .flex_1()
                        .rounded(px(8.))
                        .px(px(14.))
                        .py(px(9.))
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .bg(rgb(0x262626))
                        .text_color(rgb(theme::TEXT_MUTED))
                        .cursor_pointer()
                        .hover(|style| style.bg(rgb(0x323232)).text_color(gpui::white()))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.start_install(false, cx);
                        }))
                        .child("Install anyway"),
                )
                .child(
                    div()
                        .id("installer-primary")
                        .flex()
                        .items_center()
                        .justify_center()
                        .flex_1()
                        .rounded(px(8.))
                        .px(px(14.))
                        .py(px(9.))
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .bg(rgb(theme::ACCENT))
                        .cursor_pointer()
                        .hover(|style| style.bg(rgb(theme::ACCENT_HOVER)))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.start_install(true, cx);
                        }))
                        .child(action),
                )
                .into_any_element()
        } else {
            div()
                .id("installer-primary")
                .flex()
                .items_center()
                .justify_center()
                .w_full()
                .rounded(px(8.))
                .px(px(20.))
                .py(px(9.))
                .text_size(px(12.5))
                .line_height(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .bg(rgb(theme::ACCENT))
                .cursor_pointer()
                .hover(|style| style.bg(rgb(theme::ACCENT_HOVER)))
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    if this.choice == Choice::Portable {
                        this.portable(window, cx);
                    } else {
                        this.install(cx);
                    }
                }))
                .child(action)
                .into_any_element()
        };

        self.motion.end_frame();
        if self.motion.is_active() && !self.animation_frame_pending {
            self.animation_frame_pending = true;
            let view = cx.weak_entity();
            window.on_next_frame(move |_, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.animation_frame_pending = false;
                    cx.notify();
                });
            });
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .rounded(px(8.))
            .overflow_hidden()
            .border_1()
            .border_color(rgb(0x2a2a2a))
            .bg(rgb(theme::BG))
            .text_color(gpui::white())
            .font_family("Poppins")
            .font_weight(FontWeight::NORMAL)
            .child(
                div()
                    .flex()
                    .items_center()
                    .flex_shrink_0()
                    .h(px(40.))
                    .pl(px(18.))
                    .pr(px(10.))
                    .child(
                        div()
                            .id("installer-drag-region")
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .flex_1()
                            .h_full()
                            .window_control_area(WindowControlArea::Drag)
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.start_window_move();
                            })
                            .child(icons::grouped().w(px(18.)).h(px(18.)))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(theme::TEXT_MUTED))
                                    .child("SteamVR Lighthouse Manager"),
                            )
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(rgb(theme::TEXT_DIMMEST))
                                    .child(format!("v{version}")),
                            ),
                    )
                    .child(
                        div()
                            .id("installer-close")
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(28.))
                            .rounded(px(6.))
                            .when(!busy, |this| {
                                this.cursor_pointer()
                                    .hover(|style| style.bg(rgb(0x2c2c2c)))
                            })
                            .when(busy, |this| this.opacity(0.35))
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                if !this.busy {
                                    cx.quit();
                                }
                            }))
                            .child(
                                icons::close()
                                    .w(px(16.))
                                    .h(px(16.))
                                    .text_color(rgb(theme::TEXT_MUTED)),
                            ),
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
                    .px(px(18.))
                    .pt(px(2.))
                    .pb(px(12.))
                    .gap(px(12.))
                    .when(update_only, |this| {
                        this.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(12.))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .flex_shrink_0()
                                        .size(px(36.))
                                        .rounded(px(9.))
                                        .bg(theme::c_a(theme::ACCENT, 0.16))
                                        .child(
                                            icons::download()
                                                .w(px(18.))
                                                .h(px(18.))
                                                .text_color(rgb(theme::ACCENT_HOVER)),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .flex_1()
                                        .min_w(px(0.))
                                        .gap(px(2.))
                                        .child(
                                            div()
                                                .text_size(px(16.))
                                                .line_height(px(22.))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .child("Updating Lighthouse Manager"),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.5))
                                                .line_height(px(16.))
                                                .text_color(rgb(theme::TEXT_DIM))
                                                .child("Replacing the previous Go version while keeping your settings."),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .w_full()
                                .min_w(px(0.))
                                .px(px(8.))
                                .py(px(5.))
                                .rounded(px(6.))
                                .bg(rgb(theme::INPUT_BG))
                                .border_1()
                                .border_color(theme::c(0x262626))
                                .text_size(px(10.5))
                                .line_height(px(14.))
                                .child(
                                    div()
                                        .flex_shrink_0()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(rgb(theme::TEXT_DIM))
                                        .child("Path"),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w(px(0.))
                                        .truncate()
                                        .text_color(rgb(theme::TEXT_MUTED))
                                        .child(self.destination.clone()),
                                ),
                        )
                    })
                    .when(!update_only, |this| {
                        this.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(18.))
                                        .line_height(px(24.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child("Choose how to run"),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(17.))
                                        .text_color(rgb(theme::TEXT_DIM))
                                        .child("Install for quick access, or run portable without changing anything."),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(8.))
                                .child(self.choice_row(Choice::Install, cx))
                                .child(self.choice_row(Choice::Portable, cx)),
                        )
                    })
                    .when_some(self.error.clone(), |this, error| {
                        this.child(
                            div()
                                .rounded(px(8.))
                                .border_1()
                                .border_color(theme::c_a(theme::RED, 0.5))
                                .bg(theme::red_badge())
                                .px(px(10.))
                                .py(px(8.))
                                .text_size(px(11.))
                                .line_height(px(16.))
                                .text_color(rgb(theme::RED))
                                .child(div().font_weight(FontWeight::MEDIUM).child(error)),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .h(px(56.))
                    .px(px(18.))
                    .border_t_1()
                    .border_color(rgb(0x262626))
                    .bg(rgb(0x151515))
                    .child(footer),
            )
    }
}
