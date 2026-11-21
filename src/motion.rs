//! Interruptible, frame-driven transitions shared by every UI surface.
//! Motion defaults match framer-motion 12.9.7; CSS curves match Tailwind/browser easing.

use crate::app::App;
use gpui::{prelude::*, Context, Div, Hsla, MouseButton, Rgba, SharedString, Stateful};
use std::{collections::{HashMap, HashSet}, time::{Duration, Instant}};

#[derive(Clone, Copy)]
enum Curve { Bezier(f32, f32, f32, f32), Linear, Critical, Transform }

#[derive(Clone, Copy)]
pub struct Spec {
    duration: Duration,
    delay: Duration,
    curve: Curve,
    repeating: bool,
}

impl Spec {
    fn new(ms: u64, curve: Curve) -> Self {
        Self { duration: Duration::from_millis(ms), delay: Duration::ZERO, curve, repeating: false }
    }
    pub fn motion(ms: u64) -> Self { Self::new(ms, Curve::Bezier(0.25, 0.1, 0.35, 1.)) }
    pub fn css(ms: u64) -> Self { Self::new(ms, Curve::Bezier(0.4, 0., 0.2, 1.)) }
    pub fn ease(ms: u64) -> Self { Self::new(ms, Curve::Bezier(0.25, 0.1, 0.25, 1.)) }
    pub fn linear(ms: u64) -> Self { Self::new(ms, Curve::Linear) }
    pub fn spring(ms: u64) -> Self { Self::new(ms, Curve::Critical) }
    pub fn transform() -> Self { Self::new(2000, Curve::Transform) }
    pub fn delay(mut self, seconds: f32) -> Self {
        self.delay = Duration::from_secs_f32(seconds.max(0.));
        self
    }
    pub fn repeat(mut self) -> Self { self.repeating = true; self }
}

trait Value: Copy + PartialEq {
    fn mix(from: Self, to: Self, t: f32) -> Self;
    fn distance(self, other: Self) -> f32;
}
impl Value for f32 {
    fn mix(from: Self, to: Self, t: f32) -> Self { from + (to - from) * t }
    fn distance(self, other: Self) -> f32 { (self - other).abs() }
}
impl Value for Rgba {
    fn mix(from: Self, to: Self, t: f32) -> Self {
        // Premultiplied RGB avoids hue detours and dark fringes when alpha changes.
        let a = f32::mix(from.a, to.a, t);
        if a <= f32::EPSILON {
            return Self { r: 0., g: 0., b: 0., a: 0. };
        }
        Self {
            r: f32::mix(from.r * from.a, to.r * to.a, t) / a,
            g: f32::mix(from.g * from.a, to.g * to.a, t) / a,
            b: f32::mix(from.b * from.a, to.b * to.a, t) / a,
            a,
        }
    }
    fn distance(self, other: Self) -> f32 {
        (self.r - other.r).abs().max((self.g - other.g).abs())
            .max((self.b - other.b).abs()).max((self.a - other.a).abs())
    }
}

struct ColorTrack {
    target: Hsla,
    animation: Track<Rgba>,
}

struct Track<T> {
    from: T,
    target: T,
    start: Instant,
    spec: Spec,
    seen: u64,
    done: bool,
}
impl<T: Value> Track<T> {
    fn new(initial: T, target: T, spec: Spec, now: Instant, frame: u64) -> Self {
        Self { from: initial, target, start: now + spec.delay, spec, seen: frame, done: initial == target }
    }
    fn sample(&self, now: Instant) -> (T, bool) {
        if self.from == self.target { return (self.target, true); }
        if now < self.start { return (self.from, false); }
        let seconds = now.duration_since(self.start).as_secs_f32();
        let duration = self.spec.duration.as_secs_f32().max(f32::EPSILON);
        if matches!(self.spec.curve, Curve::Transform) {
            // Analytical mass-1 spring: stiffness 500, damping 25, restSpeed 10.
            let decay = (-12.5 * seconds).exp();
            let frequency = (500_f32 - 12.5 * 12.5).sqrt();
            let angle = frequency * seconds;
            let remaining = decay * (angle.cos() + 12.5 / frequency * angle.sin());
            let velocity = decay * 500. / frequency * angle.sin();
            let distance = self.from.distance(self.target);
            let rest_delta = if distance < 5. { 0.005 } else { 0.5 };
            let done = (remaining.abs() * distance <= rest_delta && velocity.abs() * distance <= 10.) || seconds >= duration;
            return if done { (self.target, true) } else { (T::mix(self.from, self.target, 1. - remaining), false) };
        }
        if !self.spec.repeating && seconds >= duration { return (self.target, true); }
        let progress = if self.spec.repeating { (seconds / duration).fract() } else { seconds / duration };
        let eased = match self.spec.curve {
            Curve::Linear => progress,
            Curve::Bezier(x1,y1,x2,y2) => bezier(progress, x1,y1,x2,y2),
            Curve::Critical => {
                // Motion's duration-derived critical spring: (1 + q) * exp(-q) = .001.
                let q = 9.233_413 * progress;
                1. - (1. + q) * (-q).exp()
            }
            Curve::Transform => unreachable!(),
        };
        (T::mix(self.from, self.target, eased), false)
    }
    fn update(&mut self, target: T, spec: Spec, now: Instant, frame: u64) -> (T, bool, bool) {
        if self.target != target {
            self.from = self.sample(now).0;
            self.target = target;
            self.start = now + spec.delay;
            self.spec = spec;
            self.done = self.from == target;
        }
        self.seen = frame;
        let (value, done) = self.sample(now);
        let finished = !self.done && done;
        self.done = done;
        (value, !done, finished)
    }
}

fn bezier(x: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    if x <= 0. || x >= 1. { return x.clamp(0., 1.); }
    let coordinate = |t: f32, a: f32, b: f32| 3. * (1.-t).powi(2) * t * a + 3. * (1.-t) * t*t * b + t*t*t;
    let (mut low, mut high) = (0., 1.);
    for _ in 0..16 {
        let t = (low + high) * 0.5;
        if coordinate(t, x1, x2) < x { low = t; } else { high = t; }
    }
    coordinate((low + high) * 0.5, y1, y2)
}

pub struct Swap { pub value: bool, pub opacity: f32 }
struct SwapTrack { value: bool, requested: bool, fade: Track<f32>, exiting: bool, seen: u64 }
#[derive(Default)]
struct ElementTracks {
    seen: u64,
    scalars: HashMap<&'static str, Track<f32>>,
    colors: HashMap<&'static str, ColorTrack>,
    swap: Option<SwapTrack>,
}

pub struct Motion {
    now: Instant,
    frame: u64,
    active: bool,
    layout_changed: bool,
    mount_deadline: Option<Instant>,
    elements: HashMap<String, ElementTracks>,
    hovered: HashSet<SharedString>,
    pressed: HashSet<SharedString>,
}
impl Default for Motion {
    fn default() -> Self {
        Self { now: Instant::now(), frame: 0, active: false, layout_changed: false,
            mount_deadline: None,
            elements: HashMap::new(), hovered: HashSet::new(), pressed: HashSet::new() }
    }
}
impl Motion {
    /// Fit incoming animations within the page deadline, including frame scheduling.
    pub fn begin_navigation_mount(&mut self, deadline: Instant) {
        self.mount_deadline = Some(deadline);
    }
    fn mount_spec(&self, mut spec: Spec) -> Spec {
        if let Some(deadline) = self.mount_deadline {
            let budget = deadline.saturating_duration_since(self.now);
            if !spec.repeating {
                spec.duration = spec.duration.min(budget);
                spec.delay = Duration::ZERO;
                if matches!(spec.curve, Curve::Transform) {
                    spec.curve = Curve::Bezier(0.25, 0.1, 0.35, 1.);
                }
            }
        }
        spec
    }
    pub fn begin_frame(&mut self, now: Instant) {
        self.now = now;
        self.frame = self.frame.wrapping_add(1);
        self.active = false;
        self.layout_changed = false;
    }
    fn with_element<R>(&mut self, id: &str, operation: impl FnOnce(&mut ElementTracks) -> R) -> R {
        if let Some(element) = self.elements.get_mut(id) {
            element.seen = self.frame;
            return operation(element);
        }
        let element = self.elements.entry(id.to_owned()).or_default();
        element.seen = self.frame;
        operation(element)
    }
    pub fn scalar(&mut self, id: &str, channel: &'static str, initial: f32, target: f32, spec: Spec) -> f32 {
        let spec = self.mount_spec(spec);
        let (now, frame) = (self.now, self.frame);
        let (value, active, _) = self.with_element(id, |element| {
            element.scalars.entry(channel).or_insert_with(|| Track::new(initial, target, spec, now, frame))
                .update(target, spec, now, frame)
        });
        self.active |= active;
        value
    }
    pub fn color(&mut self, id: &str, channel: &'static str, initial: Hsla, target: Hsla, spec: Spec) -> Hsla {
        let spec = self.mount_spec(spec);
        let (now, frame) = (self.now, self.frame);
        let (value, active) = self.with_element(id, |element| {
            let track = element.colors.entry(channel).or_insert_with(|| ColorTrack {
                target,
                animation: Track::new(initial.into(), target.into(), spec, now, frame),
            });
            let target_rgb = if track.target != target {
                track.target = target;
                Rgba::from(target)
            } else {
                track.animation.target
            };
            let (value, active, _) = track.animation.update(target_rgb, spec, now, frame);
            (if active { Hsla::from(value) } else { target }, active)
        });
        self.active |= active;
        value
    }
    pub fn presence(&mut self, id: &str, visible: bool, spec: Spec) -> f32 {
        self.presence_from(id, visible, 0., spec)
    }
    pub fn presence_from(&mut self, id: &str, visible: bool, initial: f32, spec: Spec) -> f32 {
        let spec = self.mount_spec(spec);
        let (now, frame) = (self.now, self.frame);
        let target = if visible { 1. } else { 0. };
        let (value, active, finished) = self.with_element(id, |element| {
            element.scalars.entry("presence").or_insert_with(|| Track::new(initial, target, spec, now, frame))
                .update(target, spec, now, frame)
        });
        self.active |= active;
        self.layout_changed |= finished && !visible;
        value.clamp(0., 1.)
    }
    pub fn swap(&mut self, id: &str, value: bool, spec: Spec) -> Swap {
        let spec = self.mount_spec(spec);
        let (now, frame) = (self.now, self.frame);
        let (result, active, changed) = self.with_element(id, |element| {
            let track = element.swap.get_or_insert_with(|| SwapTrack {
                value, requested: value, fade: Track::new(0., 1., spec, now, frame), exiting: false, seen: frame,
            });
            track.seen = frame;
            track.requested = value;
            track.exiting = track.value != value;
            let target = if track.exiting { 0. } else { 1. };
            let (mut opacity, mut active, _) = track.fade.update(target, spec, now, frame);
            let mut changed = false;
            if track.fade.done && track.exiting {
                track.value = track.requested;
                track.exiting = false;
                track.fade = Track::new(0., 1., spec, now, frame);
                opacity = 0.;
                active = true;
                changed = true;
            }
            (Swap { value: track.value, opacity: opacity.clamp(0.,1.) }, active, changed)
        });
        self.active |= active;
        self.layout_changed |= changed;
        result
    }
    pub fn spin(&mut self, id: &str, period_ms: u64) -> f32 {
        self.scalar(id, "spin", 0., 1., Spec::linear(period_ms).repeat())
    }
    pub fn settled(&self, id: &str, channel: &'static str) -> bool {
        self.elements.get(id).and_then(|e| e.scalars.get(channel)).is_none_or(|track| track.done)
    }
    pub fn is_active(&self) -> bool { self.active }
    pub fn take_layout_change(&mut self) -> bool { std::mem::take(&mut self.layout_changed) }
    pub fn is_hovered(&self, id: &str) -> bool { self.hovered.contains(id) }
    pub fn is_pressed(&self, id: &str) -> bool { self.pressed.contains(id) }
    fn set_hovered(&mut self, id: SharedString, value: bool) {
        if value { self.hovered.insert(id); } else { self.hovered.remove(&id); }
    }
    fn set_pressed(&mut self, id: SharedString, value: bool) {
        if value { self.pressed.insert(id); } else { self.pressed.remove(&id); }
    }
    pub fn clear_content(&mut self) {
        self.elements.retain(|id, _| id.starts_with("chrome:"));
        self.hovered.retain(|id| id.starts_with("chrome:"));
        self.pressed.retain(|id| id.starts_with("chrome:"));
    }
    pub fn end_frame(&mut self) {
        self.mount_deadline = None;
        let frame = self.frame;
        self.elements.retain(|_, element| {
            if element.seen != frame { return false; }
            element.scalars.retain(|_, track| track.seen == frame);
            element.colors.retain(|_, track| track.animation.seen == frame);
            if element.swap.as_ref().is_some_and(|track| track.seen != frame) { element.swap = None; }
            true
        });
    }
}

pub fn interactive(element: Stateful<Div>, id: SharedString, cx: &mut Context<App>) -> Stateful<Div> {
    let hover_id = id.clone();
    let down_id = id.clone();
    let up_id = id.clone();
    element
        .on_hover(cx.listener(move |app, hovered: &bool, _, cx| {
            app.motion.set_hovered(hover_id.clone(), *hovered);
            cx.notify();
        }))
        .on_mouse_down(MouseButton::Left, cx.listener(move |app, _, _, cx| {
            app.motion.set_pressed(down_id.clone(), true);
            cx.notify();
        }))
        .on_mouse_up(MouseButton::Left, cx.listener(move |app, _, _, cx| {
            app.motion.set_pressed(up_id.clone(), false);
            cx.notify();
        }))
        .on_mouse_up_out(MouseButton::Left, cx.listener(move |app, _, _, cx| {
            if app.motion.is_pressed(&id) {
                app.motion.set_pressed(id.clone(), false);
                cx.notify();
            }
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reversing_a_hover_transition_preserves_its_current_value() {
        let start = Instant::now();
        let mut track = Track::new(0., 1., Spec::css(200), start, 1);
        let middle = start + Duration::from_millis(80);
        let before = track.sample(middle).0;
        let after = track.update(0., Spec::css(200), middle, 2).0;
        assert!((before - after).abs() < 0.00001);
        assert!(after > 0. && after < 1.);
        assert_eq!(track.sample(middle + Duration::from_millis(200)).0, 0.);
    }

    #[test]
    fn presence_retains_exit_and_wait_swap_does_not_replace_text_early() {
        let start = Instant::now();
        let mut motion = Motion::default();
        motion.begin_frame(start);
        motion.presence("button", true, Spec::motion(300));
        motion.swap("label", false, Spec::motion(300));
        motion.begin_frame(start + Duration::from_millis(300));
        assert_eq!(motion.presence("button", true, Spec::motion(300)), 1.);
        motion.swap("label", false, Spec::motion(300));
        motion.begin_frame(start + Duration::from_millis(310));
        assert_eq!(motion.presence("button", false, Spec::motion(300)), 1.);
        assert!(!motion.swap("label", true, Spec::motion(300)).value);
        motion.begin_frame(start + Duration::from_millis(450));
        assert!(motion.presence("button", false, Spec::motion(300)) > 0.);
        assert!(!motion.swap("label", true, Spec::motion(300)).value);
        motion.begin_frame(start + Duration::from_millis(610));
        assert_eq!(motion.presence("button", false, Spec::motion(300)), 0.);
        let swapped = motion.swap("label", true, Spec::motion(300));
        assert!(swapped.value);
        assert_eq!(swapped.opacity, 0.);
    }

    #[test]
    fn delayed_rows_wait_and_spring_finishes_at_the_actual_target() {
        let start = Instant::now();
        let track = Track::new(-30., 0., Spec::transform().delay(0.15), start, 0);
        assert_eq!(track.sample(start + Duration::from_millis(149)).0, -30.);
        assert!(track.sample(start + Duration::from_millis(250)).0 > -30.);
        assert_eq!(track.sample(start + Duration::from_secs(3)), (0., true));
    }
}

#[cfg(test)]
mod interruption_tests {
    use super::*;

    #[test]
    fn a_resize_reversal_starts_at_the_displayed_height_and_finishes_exactly() {
        let start = Instant::now();
        let mut height = Track::new(418., 530., Spec::css(220), start, 0);
        let middle = start + Duration::from_millis(110);
        let before = height.sample(middle).0;
        assert!(before > 482.);
        assert_eq!(height.update(482., Spec::css(220), middle, 1).0, before);
        assert_eq!(height.sample(middle + Duration::from_millis(220)), (482., true));
    }

    #[test]
    fn swapping_before_the_first_paint_does_not_leave_an_invisible_old_label() {
        let mut motion = Motion::default();
        let now = Instant::now();
        motion.begin_frame(now);
        motion.swap("label", false, Spec::motion(300));
        let swapped = motion.swap("label", true, Spec::motion(300));
        assert!(swapped.value);
        assert_eq!(swapped.opacity, 0.);
    }
}

#[cfg(test)]
mod color_tests {
    use super::*;

    #[test]
    fn a_blue_to_gray_toggle_does_not_pass_through_green() {
        let middle = Rgba::mix(gpui::rgb(0x1d81ff), gpui::rgb(0x434343), 0.5);
        assert!(middle.b > middle.g && middle.g > middle.r);
    }

    #[test]
    fn fading_a_transparent_choice_background_preserves_its_color() {
        let middle = Rgba::mix(
            Rgba { r: 0., g: 0., b: 0., a: 0. },
            Rgba { r: 1., g: 1., b: 1., a: 1. },
            0.5,
        );
        assert_eq!((middle.r, middle.g, middle.b, middle.a), (1., 1., 1., 0.5));
    }
}

#[cfg(test)]
mod row_presence_tests {
    use super::*;

    #[test]
    fn returning_rows_are_immediately_visible_but_still_animate_removal() {
        let mut motion = Motion::default();
        let start = Instant::now();
        motion.begin_frame(start);
        assert_eq!(motion.presence_from("row", true, 1., Spec::motion(300)), 1.);
        motion.begin_frame(start + Duration::from_millis(100));
        assert_eq!(motion.presence_from("row", false, 1., Spec::motion(300)), 1.);
        motion.begin_frame(start + Duration::from_millis(250));
        let fading = motion.presence_from("row", false, 1., Spec::motion(300));
        assert!(fading > 0. && fading < 1.);
        motion.begin_frame(start + Duration::from_millis(400));
        assert_eq!(motion.presence_from("row", false, 1., Spec::motion(300)), 0.);
        assert!(motion.take_layout_change());
    }
}

#[cfg(test)]
mod navigation_tests {
    use super::*;

    #[test]
    fn incoming_content_finishes_with_the_page_without_shortening_later_interactions() {
        let start = Instant::now();
        let mut motion = Motion::default();
        motion.begin_navigation_mount(start + Duration::from_millis(150));
        motion.begin_frame(start + Duration::from_millis(20));
        motion.presence("control", true, Spec::motion(300).delay(0.1));
        motion.scalar("view", "x", -5., 0., Spec::transform());
        motion.end_frame();
        motion.begin_frame(start + Duration::from_millis(150));
        assert_eq!(motion.presence("control", true, Spec::motion(300)), 1.);
        assert_eq!(motion.scalar("view", "x", -5., 0., Spec::transform()), 0.);
        motion.end_frame();
        motion.begin_frame(start + Duration::from_millis(151));
        motion.presence("control", false, Spec::motion(300));
        motion.end_frame();
        motion.begin_frame(start + Duration::from_millis(301));
        let fading = motion.presence("control", false, Spec::motion(300));
        assert!(fading > 0. && fading < 1.);
    }

    #[test]
    fn navigation_does_not_speed_up_the_scan_spinner() {
        let start = Instant::now();
        let mut motion = Motion::default();
        motion.begin_navigation_mount(start + Duration::from_millis(150));
        motion.begin_frame(start);
        motion.spin("scan", 1000);
        motion.end_frame();
        motion.begin_frame(start + Duration::from_millis(500));
        assert!((motion.spin("scan", 1000) - 0.5).abs() < 0.0001);
    }
}
