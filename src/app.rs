//! Root view: owns the routing, transient UI state, and the shared `AppState`.

use crate::components::{container_title_bar, title_bar};
use crate::components::text_input::TextInput;
use crate::models::{Breadcrumb, Route};
use crate::motion::{Motion, Spec};
use crate::state::AppState;
use gpui::{prelude::*, 
    Context, Entity, IntoElement, Render, Subscription, Window, div,
    px, size,
};
use std::collections::HashSet;
use std::time::{Duration, Instant};

const NAVIGATION_MS: u64 = 300;

enum PendingRemoval {
    Station(String),
    Group(String),
}

pub struct App {
    pub state: Entity<AppState>,
    pub route: Route,
    pub breadcrumbs: Vec<Breadcrumb>,
    pub edit_mode: bool,
    pub selected: Vec<String>,
    pub channel_dropdown_open: bool,
    pub language_dropdown_open: bool,
    pub update_dropdown_open: bool,
    pub identify_cooldown: HashSet<String>,
    pub nickname_input: Entity<TextInput>,
    pub editing_station: Option<String>,
    pub editing_group: Option<String>,
    pub language: String,
    pub motion: Motion,
    pub view_transitions: crate::views::ViewTransitions,
    pub breadcrumb_transitions: container_title_bar::BreadcrumbTransitions,
    pending_route: Option<Route>,
    navigation_deadline: Option<Instant>,
    pending_removal: Option<PendingRemoval>,
    resize_target: Option<f32>,
    resize_deadline: Option<Instant>,
    animation_frame_pending: bool,
    resize_pending: bool,
    layout_revision: u64,
    #[cfg(target_os = "windows")]
    tray: Option<crate::backend::tray::Tray>,
    #[cfg(target_os = "windows")]
    tray_steamvr_active: bool,
    _subscriptions: Vec<Subscription>,
}

impl App {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| AppState::new());

        // Reusable single-line text input for the nickname / group-name editors.
        let nickname_input = cx.new(|cx| TextInput::new(cx, String::new(), 16));

        // Drain BLE events from the backend thread.
        let event_rx = state.read(cx).ble_events();
        let state_entity = state.clone();
        let mut async_app = cx.to_async();
        gpui::App::spawn(cx, async move |_| {
            while let Ok(event) = event_rx.recv_async().await {
                let _ = state_entity.update(&mut async_app, |s, cx| {
                    s.apply_ble_event(event);
                    cx.notify();
                });
            }
        })
        .detach();

        if let Some(events) = state.read(cx).steamvr_events() {
            let state_entity = state.clone();
            let mut async_app = cx.to_async();
            gpui::App::spawn(cx, async move |_| {
                while let Ok(event) = events.recv_async().await {
                    let _ = state_entity.update(&mut async_app, |s, cx| {
                        s.apply_steamvr_event(event);
                        cx.notify();
                    });
                }
            })
            .detach();
        }

        if let Some(events) = state.read(cx).update_events() {
            let this = cx.weak_entity();
            let mut async_app = cx.to_async();
            gpui::App::spawn(cx, async move |_| {
                while let Ok(event) = events.recv_async().await {
                    let restarting = matches!(event, crate::backend::updates::Event::ReadyToRestart);
                    if this.update(&mut async_app, |app, cx| {
                        match event {
                            crate::backend::updates::Event::Status(status) => {
                                app.state.update(cx, |state, cx| {
                                    state.apply_update_status(status);
                                    cx.notify();
                                });
                            }
                            crate::backend::updates::Event::ReadyToRestart => {
                                // Update handoff must exit, never use the close-to-tray path.
                                #[cfg(target_os = "windows")]
                                app.tray.take();
                                cx.quit();
                            }
                        }
                    }).is_err() || restarting {
                        break;
                    }
                }
            }).detach();
        }

        let mut app = App {
            state,
            route: Route::Devices,
            breadcrumbs: vec![Breadcrumb {
                text: "Devices".into(),
                route: Route::Devices,
            }],
            edit_mode: false,
            selected: Vec::new(),
            channel_dropdown_open: false,
            language_dropdown_open: false,
            update_dropdown_open: false,
            identify_cooldown: HashSet::new(),
            nickname_input,
            editing_station: None,
            editing_group: None,
            language: "en".to_string(),
            motion: Motion::default(),
            view_transitions: Default::default(),
            breadcrumb_transitions: Default::default(),
            pending_route: None,
            navigation_deadline: None,
            pending_removal: None,
            resize_target: None,
            resize_deadline: None,
            animation_frame_pending: false,
            resize_pending: true,
            layout_revision: 0,
            #[cfg(target_os = "windows")]
            tray: None,
            #[cfg(target_os = "windows")]
            tray_steamvr_active: false,
            _subscriptions: Vec::new(),
        };

        // Commit text changes from the reusable input into the active editor.
        let nickname_input = app.nickname_input.clone();
        app._subscriptions.push(cx.subscribe(&nickname_input, move |this, input, _: &crate::components::text_input::InputChanged, cx| {
            let text = input.read(cx).content();
            this.commit_text(text, cx);
        }));

        // Re-render when the shared state changes (BLE events, config, etc).
        let state = app.state.clone();
        app._subscriptions.push(cx.observe(&state, move |this, _, cx| {
            this.request_resize();
            #[cfg(target_os = "windows")]
            this.sync_tray(cx);
            cx.notify();
        }));

        app
    }

    #[cfg(target_os = "windows")]
    pub fn initialize_tray(&mut self, window: &Window, cx: &mut Context<Self>) {
        match crate::backend::tray::Tray::new(window) {
            Ok(tray) => {
                let events = tray.events.clone();
                self.tray = Some(tray);
                self.sync_tray(cx);
                let this = cx.weak_entity();
                let mut async_app = cx.to_async();
                gpui::App::spawn(cx, async move |_| {
                    while let Ok(action) = events.recv_async().await {
                        if this.update(&mut async_app, |app, cx| {
                            app.handle_tray_action(action, cx);
                        }).is_err() {
                            break;
                        }
                    }
                }).detach();
            }
            Err(error) => log::error!("Cannot initialize Windows tray: {error:#}"),
        }
    }

    pub fn tray_available(&self) -> bool {
        #[cfg(target_os = "windows")]
        { self.tray.is_some() }
        #[cfg(not(target_os = "windows"))]
        { false }
    }

    #[cfg(target_os = "windows")]
    fn sync_tray(&mut self, cx: &mut Context<Self>) {
        let state = self.state.read(cx);
        let allowed = state.config.allow_tray;
        let active = allowed && state.config.is_steamvr_managed
            && state.steamvr_state_known && state.steamvr_active;
        if let Some(tray) = self.tray.as_mut() {
            if let Err(error) = tray.set_enabled(allowed) {
                log::error!("Cannot update tray visibility: {error}");
                // Never hide a window without a usable way to restore it.
                tray.show();
                return;
            }
            if active && !self.tray_steamvr_active {
                tray.hide();
            }
        }
        self.tray_steamvr_active = active;
    }

    #[cfg(target_os = "windows")]
    fn handle_tray_action(&mut self, action: crate::backend::tray::Action, cx: &mut Context<Self>) {
        use crate::backend::tray::Action;
        log::info!("Tray action: {action:?}");
        match action {
            Action::Show => {
                if let Some(tray) = self.tray.as_mut() {
                    tray.show();
                }
            }
            Action::Wake => self.set_all_power("awake", cx),
            Action::Sleep => self.set_all_power("sleep", cx),
            Action::Quit => {
                self.tray.take();
                cx.quit();
            }
        }
    }

    pub fn t(&self, key: &str) -> String {
        crate::i18n::t(&self.language, key)
    }

    /// Disable repeated identify requests for a station for 20 seconds.
    pub fn begin_identify_cooldown(&mut self, id: String, cx: &mut Context<Self>) {
        self.identify_cooldown.insert(id.clone());
        let this = cx.weak_entity();
        let mut async_app = cx.to_async();
        gpui::App::spawn(cx, async move |_| {
            gpui::Timer::after(Duration::from_secs(20)).await;
            this.update(&mut async_app, |this, cx| {
                this.identify_cooldown.remove(&id);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn navigate(&mut self, route: Route, cx: &mut Context<Self>) {
        if route == self.route && self.pending_removal.is_none() {
            self.pending_route = None;
            self.navigation_deadline = None;
        } else {
            if self.pending_route.is_none() {
                self.navigation_deadline = Some(Instant::now() + Duration::from_millis(NAVIGATION_MS));
            }
            self.pending_route = Some(route);
        }
        cx.notify();
    }

    /// Commit only after the outgoing view finishes its exit, matching wait-mode presence.
    fn commit_route(&mut self, route: Route, cx: &mut Context<Self>) {
        if let Some(removal) = self.pending_removal.take() {
            self.state.update(cx, |state, _| match removal {
                PendingRemoval::Station(id) => state.forget(&id),
                PendingRemoval::Group(id) => state.remove_group(&id),
            });
        }
        self.motion.clear_content();
        if let Some(deadline) = self.navigation_deadline {
            self.motion.begin_navigation_mount(deadline);
        }
        self.resize_target = None;
        self.resize_deadline = self.navigation_deadline;
        self.view_transitions = crate::views::ViewTransitions::for_navigation();
        self.editing_station = None;
        self.editing_group = None;
        self.route = route.clone();
        self.channel_dropdown_open = false;
        self.language_dropdown_open = false;
        self.update_dropdown_open = false;
        self.edit_mode = false;
        self.selected.clear();

        self.breadcrumbs = match &route {
            Route::Devices => vec![Breadcrumb {
                text: self.t("Devices"),
                route: Route::Devices,
            }],
            Route::SoftwareSettings => vec![
                Breadcrumb {
                    text: "SteamVR LM".into(),
                    route: Route::Devices,
                },
                Breadcrumb {
                    text: self.t("Settings"),
                    route: Route::SoftwareSettings,
                },
            ],
            Route::Updates => vec![
                Breadcrumb {
                    text: "SteamVR LM".into(),
                    route: Route::Devices,
                },
                Breadcrumb {
                    text: self.t("Settings"),
                    route: Route::SoftwareSettings,
                },
                Breadcrumb {
                    text: self.t("Updates"),
                    route: Route::Updates,
                },
            ],
            Route::BaseStationSettings(id) => {
                self.editing_station = Some(id.clone());
                self.editing_group = None;
                let name = self
                    .state
                    .read(cx)
                    .lighthouses
                    .get(id)
                    .filter(|bs| bs.name != *id)
                    .map(|bs| bs.name.clone())
                    .unwrap_or_default();
                self.nickname_input.update(cx, |input, cx| {
                    input.set_content(name);
                    input.set_placeholder(id.clone());
                    cx.notify();
                });
                vec![
                    Breadcrumb {
                        text: self.t("Devices"),
                        route: Route::Devices,
                    },
                    Breadcrumb {
                        text: id.clone(),
                        route: route.clone(),
                    },
                ]
            }
            Route::Group(id) => {
                self.editing_group = Some(id.clone());
                self.editing_station = None;
                let name = self
                    .state
                    .read(cx)
                    .config
                    .groups
                    .get(id)
                    .map(|g| g.name.clone())
                    .unwrap_or_default();
                self.nickname_input.update(cx, |input, cx| {
                    input.set_content(name.clone());
                    input.set_placeholder(String::new());
                    cx.notify();
                });
                vec![
                    Breadcrumb {
                        text: self.t("Devices"),
                        route: Route::Devices,
                    },
                    Breadcrumb {
                        text: name,
                        route: route.clone(),
                    },
                ]
            }
            Route::GroupSettings(id) => {
                self.editing_group = Some(id.clone());
                self.editing_station = None;
                let name = self
                    .state
                    .read(cx)
                    .config
                    .groups
                    .get(id)
                    .map(|g| g.name.clone())
                    .unwrap_or_default();
                self.nickname_input.update(cx, |input, cx| {
                    input.set_content(name.clone());
                    input.set_placeholder(String::new());
                    cx.notify();
                });
                vec![
                    Breadcrumb {
                        text: self.t("Devices"),
                        route: Route::Devices,
                    },
                    Breadcrumb {
                        text: name,
                        route: Route::Group(id.clone()),
                    },
                    Breadcrumb {
                        text: self.t("Settings"),
                        route: route.clone(),
                    },
                ]
            }
        };

        self.request_resize();
        cx.notify();
    }

    /// Measure the next layout, not the previous route's viewport.
    fn request_resize(&mut self) {
        self.layout_revision = self.layout_revision.wrapping_add(1);
        self.resize_pending = true;
    }

    // ---- action handlers (used by components via AppFn callbacks) ----

    pub fn commit_text(&mut self, text: String, cx: &mut Context<Self>) {
        if let Some(id) = self.editing_group.clone() {
            self.state.update(cx, |s, cx| {
                s.rename_group(&id, text.clone());
            });
        }
        if let Some(id) = self.editing_station.clone() {
            self.state.update(cx, |s, cx| {
                s.set_nickname(&id, text);
            });
        }
        cx.notify();
    }

    pub fn toggle_power(&mut self, id: String, cx: &mut Context<Self>) {
        let status = {
            let s = self.state.read(cx);
            s.lighthouses
                .get(&id)
                .map(|bs| if bs.is_awake() { "sleep" } else { "awake" })
                .unwrap_or("awake")
                .to_string()
        };
        self.state.update(cx, |s, cx| {
            s.change_power(&id, &status);
        });
        cx.notify();
    }

    pub fn toggle_all_power(&mut self, cx: &mut Context<Self>) {
        let any_awake = self
            .state
            .read(cx)
            .lighthouses
            .values()
            .any(|bs| bs.is_awake());
        let status = if any_awake { "sleep" } else { "awake" };
        self.set_all_power(status, cx);
    }

    fn set_all_power(&mut self, status: &str, cx: &mut Context<Self>) {
        let ids: Vec<String> = self.state.read(cx).lighthouses.keys().cloned().collect();
        self.state.update(cx, |s, cx| {
            for id in ids {
                s.change_power(&id, status);
            }
        });
        cx.notify();
    }

    pub fn identify(&mut self, id: String, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| {
            s.identify(&id);
        });
        self.begin_identify_cooldown(id, cx);
        cx.notify();
    }

    pub fn toggle_edit_mode(&mut self, cx: &mut Context<Self>) {
        self.edit_mode = !self.edit_mode;
        self.selected.clear();
        cx.notify();
    }

    pub fn select_station(&mut self, id: String, cx: &mut Context<Self>) {
        if self.selected.contains(&id) {
            self.selected.retain(|s| s != &id);
        } else {
            self.selected.push(id);
        }
        cx.notify();
    }

    pub fn group_selected(&mut self, cx: &mut Context<Self>) {
        if self.selected.is_empty() {
            return;
        }
        let stations = self.selected.clone();
        let name = self.t("New Group");
        let id = self
            .state
            .update(cx, |s, cx| s.create_group(name, stations));
        self.selected.clear();
        self.navigate(Route::Group(id), cx);
    }

    pub fn start_scan(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| s.start_scan());
        cx.notify();
    }

    pub fn forget_station(&mut self, id: String, cx: &mut Context<Self>) {
        self.pending_removal = Some(PendingRemoval::Station(id));
        self.navigate(Route::Devices, cx);
    }

    pub fn set_channel(&mut self, id: String, channel: u8, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| {
            s.change_channel(&id, channel);
        });
        self.channel_dropdown_open = false;
        self.request_resize();
        cx.notify();
    }

    pub fn set_managed_flags(&mut self, id: String, flag: i32, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| s.set_managed_flags(&id, flag));
        cx.notify();
    }

    pub fn toggle_steamvr_management(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| {
            s.toggle_steamvr_management();
            cx.notify();
        });
        cx.notify();
    }

    pub fn toggle_tray(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| {
            s.toggle_tray();
            cx.notify();
        });
        cx.notify();
    }

    pub fn set_branch(&mut self, branch: String, cx: &mut Context<Self>) {
        self.update_dropdown_open = false;
        self.request_resize();
        self.state.update(cx, |s, cx| {
            s.set_branch(&branch);
            cx.notify();
        });
        cx.notify();
    }

    pub fn check_updates(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| {
            s.check_updates();
            cx.notify();
        });
    }

    pub fn install_update(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| {
            s.install_update();
            cx.notify();
        });
    }

    pub fn set_language(&mut self, lang: String, cx: &mut Context<Self>) {
        self.language = lang;
        self.language_dropdown_open = false;
        self.request_resize();
        cx.notify();
    }

    pub fn quit(&mut self, cx: &mut Context<Self>) {
        #[cfg(target_os = "windows")]
        if self.state.read(cx).config.allow_tray {
            if let Some(tray) = self.tray.as_mut().filter(|tray| tray.enabled()) {
                tray.hide();
                return;
            }
        }
        #[cfg(target_os = "windows")]
        self.tray.take();
        cx.quit();
    }

    pub fn delete_group(&mut self, id: String, cx: &mut Context<Self>) {
        self.pending_removal = Some(PendingRemoval::Group(id));
        self.navigate(Route::Devices, cx);
    }

    pub fn update_group_flags(&mut self, id: String, flag: i32, cx: &mut Context<Self>) {
        let current = self
            .state
            .read(cx)
            .config
            .groups
            .get(&id)
            .map(|g| g.managed_flags)
            .unwrap_or(0);
        let next = if (current & flag) > 0 {
            current & !flag
        } else {
            current | flag
        };
        self.state
            .update(cx, |s, cx| {
                s.update_group_flags(&id, next);
            });
        cx.notify();
    }

    pub fn toggle_channel_dropdown(&mut self, cx: &mut Context<Self>) {
        self.channel_dropdown_open = !self.channel_dropdown_open;
        self.request_resize();
        cx.notify();
    }


    pub fn toggle_language_dropdown(&mut self, cx: &mut Context<Self>) {
        self.language_dropdown_open = !self.language_dropdown_open;
        self.request_resize();
        cx.notify();
    }

    pub fn toggle_update_dropdown(&mut self, cx: &mut Context<Self>) {
        self.update_dropdown_open = !self.update_dropdown_open;
        self.request_resize();
        cx.notify();
    }

    fn route_key(&self) -> String {
        match &self.route {
            Route::Devices => "devices".into(),
            Route::SoftwareSettings => "settings".into(),
            Route::Updates => "updates".into(),
            Route::BaseStationSettings(id) => format!("devices/{id}"),
            Route::Group(id) => format!("groups/{id}"),
            Route::GroupSettings(id) => format!("groups/{id}/settings"),
        }
    }
}

impl Render for App {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = Instant::now();
        self.motion.begin_frame(now);
        let exiting = self.pending_route.is_some();
        let route_spec = Spec::motion(if self.navigation_deadline.is_some() { NAVIGATION_MS / 2 } else { NAVIGATION_MS });
        let route_opacity = self.motion.scalar("route", "opacity", 0.,
            if exiting { 0. } else { 1. }, route_spec).clamp(0., 1.);
        let route_x = self.motion.scalar("route", "x", -5.,
            if exiting { 5. } else { 0. },
            if self.navigation_deadline.is_some() { route_spec } else { Spec::transform() });
        let route_key = self.route_key();
        let view = match &self.route {
            Route::Devices => self.render_devices_view(cx),
            Route::SoftwareSettings => self.render_software_settings_view(cx),
            Route::Updates => self.render_updates_view(cx),
            Route::BaseStationSettings(_) => self.render_base_station_settings_view(cx),
            Route::Group(_) => self.render_group_view(cx),
            Route::GroupSettings(_) => self.render_group_settings_view(cx),
        };
        let content = div()
            .w_full()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap(px(12.))
            .pb(px(24.))
            .child(title_bar::render(self, cx))
            .child(container_title_bar::render(self, cx))
            .child(
                div().px(px(24.)).child(
                    div().relative().left(px(route_x)).opacity(route_opacity)
                        .child(view)
                        .when(exiting, |this| this.child(div().absolute().inset_0().occlude())),
                ),
            );

        let viewport = window.viewport_size();
        let next_height = self.resize_target.map(|target| {
            let duration = self.resize_deadline.take()
                .map(|deadline| deadline.saturating_duration_since(now).as_millis() as u64)
                .unwrap_or(220);
            self.motion.scalar("chrome:window", "height", f32::from(viewport.height),
                target, Spec::css(duration)).round()
        });
        self.motion.end_frame();
        if self.motion.take_layout_change() {
            self.request_resize();
        }
        let route_finished = exiting && self.motion.settled("route", "opacity")
            && self.motion.settled("route", "x");
        let height_changed = next_height.is_some_and(|height| (f32::from(viewport.height) - height).abs() >= 1.);
        if (self.motion.is_active() || route_finished || height_changed) && !self.animation_frame_pending {
            self.animation_frame_pending = true;
            let view = cx.weak_entity();
            window.on_next_frame(move |window, cx| {
                let _ = view.update(cx, |app, cx| {
                    app.animation_frame_pending = false;
                    if let Some(height) = next_height {
                        let viewport = window.viewport_size();
                        if (f32::from(viewport.height) - height).abs() >= 1. {
                            window.resize(size(viewport.width, px(height)));
                        }
                    }
                    if app.pending_route.is_some() && app.motion.settled("route", "opacity")
                        && app.motion.settled("route", "x")
                    {
                        let route = app.pending_route.take().unwrap();
                        app.commit_route(route, cx);
                    }
                    cx.notify();
                });
            });
        }

        let root = div()
            .size_full()
            .font_family("Poppins")
            .bg(gpui::rgb(0x181818))
            .child(content);
        if !self.resize_pending {
            return root.id(gpui::SharedString::from(route_key)).overflow_y_scroll();
        }
        let revision = self.layout_revision;
        let view = cx.weak_entity();
        root.on_children_prepainted(move |bounds, window, _cx| {
            let Some(content) = bounds.first() else { return; };
            let content_height = f32::from(content.size.height);
            let view = view.clone();
            window.on_next_frame(move |window, cx| {
                let _ = view.update(cx, |app, cx| {
                    if !app.resize_pending || app.layout_revision != revision {
                        return;
                    }
                    app.resize_pending = false;
                    let viewport = window.viewport_size();
                    let available_height = window.display(cx).map(|display| {
                        let bounds = window.bounds();
                        let frame_height = f32::from(bounds.size.height - viewport.height);
                        f32::from(display.bounds().bottom())
                            - f32::from(bounds.origin.y) - frame_height - 32.
                    }).unwrap_or(content_height);
                    app.resize_target = Some(fitted_window_height(content_height, available_height));
                    cx.notify();
                });
            });
        }).id(gpui::SharedString::from(route_key)).overflow_y_scroll()
    }
}

/// Content includes the shared page padding; oversized views remain scrollable.
fn fitted_window_height(content_height: f32, available_height: f32) -> f32 {
    content_height.ceil().min(available_height.floor().max(1.))
}

#[cfg(test)]
mod tests {
    use super::fitted_window_height;

    #[test]
    fn window_fits_each_views_content_without_extending_past_the_screen() {
        assert_eq!(fitted_window_height(310.2, 900.), 311.);
        assert_eq!(fitted_window_height(610.8, 900.), 611.);
        assert_eq!(fitted_window_height(610.8, 450.7), 450.);
    }
}
