//! Reusable GPUI components.

pub mod base_station;
pub mod base_station_group;
pub mod checkbox;
pub mod container_title_bar;
pub mod dropdown;
pub mod text_input;
pub mod title_bar;

use gpui::Context;

/// An event callback that mutates the root `App`.
pub type AppFn = Box<dyn Fn(&mut crate::app::App, &mut Context<crate::app::App>) + 'static>;

/// A callback with a single string payload (e.g. dropdown selection).
pub type AppValueFn =
    Box<dyn Fn(&mut crate::app::App, &mut Context<crate::app::App>, String) + 'static>;
