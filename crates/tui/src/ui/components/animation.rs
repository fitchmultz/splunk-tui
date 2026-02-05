//! Animation controller for visual effects.
//!
//! Provides animation effects using `tachyonfx` for smooth transitions.
//! Note: Full shader-like effects require frame-level integration.
//!
//! # Example
//!
//! ```rust,ignore
//! use splunk_tui::ui::components::{AnimationController, AnimationType};
//! use std::time::Duration;
//!
//! // Create a fade-in animation
//! let mut animation = AnimationController::fade_in(500);
//!
//! // Check progress
//! let progress = animation.progress();
//!
//! // Check if completed
//! if animation.is_completed() {
//!     // Animation done
//! }
//!
//! // Reset to replay
//! animation.reset();
//! ```

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};
use std::time::{Duration, Instant};
use tachyonfx::{Effect, EffectTimer};

/// Types of animations available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationType {
    /// Fade in from transparent to full opacity.
    FadeIn,
    /// Fade out from full opacity to transparent.
    FadeOut,
    /// Slide content up.
    SlideUp,
    /// Slide content down.
    SlideDown,
    /// Slide content left.
    SlideLeft,
    /// Slide content right.
    SlideRight,
    /// Dissolve effect.
    Dissolve,
}

/// Controls animation state and rendering.
#[derive(Debug, Clone)]
pub struct AnimationController {
    start_time: Instant,
    duration: Duration,
    animation_type: AnimationType,
    completed: bool,
    looping: bool,
}

impl AnimationController {
    /// Create a new animation controller.
    pub fn new(animation_type: AnimationType, duration: Duration) -> Self {
        Self {
            start_time: Instant::now(),
            duration,
            animation_type,
            completed: false,
            looping: false,
        }
    }

    /// Create a fade-in animation.
    pub fn fade_in(duration_ms: u64) -> Self {
        Self::new(AnimationType::FadeIn, Duration::from_millis(duration_ms))
    }

    /// Create a fade-out animation.
    pub fn fade_out(duration_ms: u64) -> Self {
        Self::new(AnimationType::FadeOut, Duration::from_millis(duration_ms))
    }

    /// Create a dissolve animation.
    pub fn dissolve(duration_ms: u64) -> Self {
        Self::new(AnimationType::Dissolve, Duration::from_millis(duration_ms))
    }

    /// Create a slide-up animation.
    pub fn slide_up(duration_ms: u64) -> Self {
        Self::new(AnimationType::SlideUp, Duration::from_millis(duration_ms))
    }

    /// Create a slide-down animation.
    pub fn slide_down(duration_ms: u64) -> Self {
        Self::new(AnimationType::SlideDown, Duration::from_millis(duration_ms))
    }

    /// Create a slide-left animation.
    pub fn slide_left(duration_ms: u64) -> Self {
        Self::new(AnimationType::SlideLeft, Duration::from_millis(duration_ms))
    }

    /// Create a slide-right animation.
    pub fn slide_right(duration_ms: u64) -> Self {
        Self::new(
            AnimationType::SlideRight,
            Duration::from_millis(duration_ms),
        )
    }

    /// Set whether the animation should loop.
    pub fn looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    /// Reset the animation to start over.
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.completed = false;
    }

    /// Check if the animation has completed.
    pub fn is_completed(&self) -> bool {
        self.completed
    }

    /// Get the animation type.
    pub fn animation_type(&self) -> AnimationType {
        self.animation_type
    }

    /// Get current progress (0.0 to 1.0).
    pub fn progress(&self) -> f32 {
        let elapsed = self.start_time.elapsed();
        if elapsed >= self.duration {
            1.0
        } else {
            elapsed.as_secs_f32() / self.duration.as_secs_f32()
        }
    }

    /// Get remaining duration.
    pub fn remaining(&self) -> Duration {
        let elapsed = self.start_time.elapsed();
        self.duration.saturating_sub(elapsed)
    }

    /// Create an effect based on animation type and progress.
    #[allow(dead_code)]
    fn create_effect(&self, _progress: f32) -> Option<Effect> {
        let timer = EffectTimer::from_ms(self.duration.as_millis() as u32, Default::default());

        match self.animation_type {
            AnimationType::FadeIn => {
                // Create a fade-in effect
                let _ = timer;
                // Note: tachyonfx 0.14 API may differ from the original code
                // Using placeholder - full implementation would need proper effect creation
                None
            }
            AnimationType::FadeOut => {
                // Fade out effect
                None
            }
            AnimationType::Dissolve => {
                // Dissolve effect
                None
            }
            _ => None, // Other animations require more complex effect composition
        }
    }

    /// Process the animation for one frame.
    /// Returns true if the animation is still running.
    pub fn tick(&mut self) -> bool {
        if self.completed {
            if self.looping {
                self.reset();
                return true;
            }
            return false;
        }

        let progress = self.progress();
        if progress >= 1.0 {
            self.completed = true;
            if self.looping {
                self.reset();
                return true;
            }
            return false;
        }

        true
    }
}

/// A widget wrapper that applies an animation effect.
#[derive(Debug, Clone)]
pub struct AnimatedWidget<W: Widget> {
    widget: W,
    animation: AnimationController,
}

impl<W: Widget> AnimatedWidget<W> {
    /// Create a new animated widget.
    pub fn new(widget: W, animation: AnimationController) -> Self {
        Self { widget, animation }
    }

    /// Get mutable access to the animation controller.
    pub fn animation_mut(&mut self) -> &mut AnimationController {
        &mut self.animation
    }

    /// Get a reference to the animation controller.
    pub fn animation(&self) -> &AnimationController {
        &self.animation
    }

    /// Get the inner widget.
    pub fn into_inner(self) -> W {
        self.widget
    }

    /// Process the animation for one frame.
    pub fn tick(&mut self) -> bool {
        self.animation.tick()
    }
}

impl<W: Widget> Widget for AnimatedWidget<W> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        // Tick the animation
        self.animation.tick();

        // For now, render the widget directly.
        // Full tachyonfx integration requires frame-level rendering which
        // needs access to the terminal backend directly.
        self.widget.render(area, buf);
    }
}

/// Simple fade-in wrapper for any widget.
#[derive(Debug, Clone)]
pub struct FadeIn<W: Widget> {
    widget: W,
    controller: AnimationController,
}

impl<W: Widget> FadeIn<W> {
    /// Create a new fade-in wrapper.
    pub fn new(widget: W, duration_ms: u64) -> Self {
        Self {
            widget,
            controller: AnimationController::fade_in(duration_ms),
        }
    }

    /// Check if the fade-in is complete.
    pub fn is_complete(&self) -> bool {
        self.controller.is_completed()
    }

    /// Reset the fade-in.
    pub fn reset(&mut self) {
        self.controller.reset();
    }

    /// Get the progress (0.0 to 1.0).
    pub fn progress(&self) -> f32 {
        self.controller.progress()
    }
}

impl<W: Widget> Widget for FadeIn<W> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.widget.render(area, buf);
    }
}

/// A simple opacity wrapper that adjusts the alpha of rendered content.
#[derive(Debug, Clone)]
pub struct Opacity<W: Widget> {
    widget: W,
    opacity: f32, // 0.0 to 1.0
}

impl<W: Widget> Opacity<W> {
    /// Create a new opacity wrapper.
    pub fn new(widget: W, opacity: f32) -> Self {
        Self {
            widget,
            opacity: opacity.clamp(0.0, 1.0),
        }
    }

    /// Set the opacity level.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Get the current opacity.
    pub fn get_opacity(&self) -> f32 {
        self.opacity
    }
}

impl<W: Widget> Widget for Opacity<W> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Render the widget first
        self.widget.render(area, buf);

        // Then adjust opacity of all cells in the area
        if self.opacity < 1.0 {
            let alpha = (self.opacity * 255.0) as u8;

            for y in area.y..(area.y + area.height) {
                for x in area.x..(area.x + area.width) {
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        // Dim the foreground color based on opacity
                        let fg = cell.style().fg.unwrap_or(Color::White);
                        cell.set_style(Style::default().fg(dimm_color(fg, alpha)));
                    }
                }
            }
        }
    }
}

/// Helper function to dim a color by an alpha value.
fn dimm_color(color: Color, alpha: u8) -> Color {
    // Simple opacity approximation - for true alpha blending,
    // we'd need access to the background color
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            ((r as u16 * alpha as u16) / 255) as u8,
            ((g as u16 * alpha as u16) / 255) as u8,
            ((b as u16 * alpha as u16) / 255) as u8,
        ),
        _ => {
            // For indexed colors, just return as-is
            // Full implementation would map to RGB and back
            color
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_controller_creation() {
        let animation = AnimationController::new(AnimationType::FadeIn, Duration::from_millis(500));

        assert_eq!(animation.animation_type(), AnimationType::FadeIn);
        assert!(!animation.is_completed());
        // Progress should be very close to 0 at creation
        assert!(
            animation.progress() < 0.01,
            "Progress should be near 0 at creation"
        );
        assert!(!animation.looping);
    }

    #[test]
    fn test_animation_controller_fade_in() {
        let animation = AnimationController::fade_in(1000);

        assert_eq!(animation.animation_type(), AnimationType::FadeIn);
    }

    #[test]
    fn test_animation_controller_fade_out() {
        let animation = AnimationController::fade_out(1000);

        assert_eq!(animation.animation_type(), AnimationType::FadeOut);
    }

    #[test]
    fn test_animation_controller_slide_variants() {
        let up = AnimationController::slide_up(500);
        assert_eq!(up.animation_type(), AnimationType::SlideUp);

        let down = AnimationController::slide_down(500);
        assert_eq!(down.animation_type(), AnimationType::SlideDown);

        let left = AnimationController::slide_left(500);
        assert_eq!(left.animation_type(), AnimationType::SlideLeft);

        let right = AnimationController::slide_right(500);
        assert_eq!(right.animation_type(), AnimationType::SlideRight);
    }

    #[test]
    fn test_animation_controller_dissolve() {
        let animation = AnimationController::dissolve(500);
        assert_eq!(animation.animation_type(), AnimationType::Dissolve);
    }

    #[test]
    fn test_animation_controller_looping() {
        let animation = AnimationController::fade_in(500).looping(true);

        assert!(animation.looping);
    }

    #[test]
    fn test_animation_controller_reset() {
        let mut animation = AnimationController::fade_in(100);

        // Let some time pass
        std::thread::sleep(Duration::from_millis(50));
        let progress_before = animation.progress();
        assert!(progress_before > 0.0, "Progress should have increased");

        // Reset
        animation.reset();
        assert!(!animation.is_completed());
        // Progress should be reset near 0
        assert!(
            animation.progress() < 0.01,
            "Progress should be near 0 after reset"
        );
    }

    #[test]
    fn test_animation_controller_tick() {
        let mut animation = AnimationController::fade_in(10000);

        assert!(animation.tick()); // Still running
        assert!(!animation.is_completed());
    }

    #[test]
    fn test_animation_controller_remaining() {
        let animation = AnimationController::fade_in(1000);

        let remaining = animation.remaining();
        assert!(remaining <= Duration::from_millis(1000));
        assert!(remaining > Duration::from_millis(0));
    }

    #[test]
    fn test_animated_widget_creation() {
        use ratatui::widgets::Block;

        let block = Block::default();
        let animation = AnimationController::fade_in(500);
        let animated = AnimatedWidget::new(block, animation);

        assert_eq!(animated.animation().animation_type(), AnimationType::FadeIn);
    }

    #[test]
    fn test_animated_widget_tick() {
        use ratatui::widgets::Block;

        let block = Block::default();
        let animation = AnimationController::fade_in(10000);
        let mut animated = AnimatedWidget::new(block, animation);

        assert!(animated.tick());
    }

    #[test]
    fn test_animated_widget_mut() {
        use ratatui::widgets::Block;

        let block = Block::default();
        let animation = AnimationController::fade_in(500);
        let mut animated = AnimatedWidget::new(block, animation);

        // Reset animation through mutable reference
        animated.animation_mut().reset();
        // Progress should be near 0 after reset
        assert!(
            animated.animation().progress() < 0.01,
            "Progress should be near 0 after reset"
        );
    }

    #[test]
    fn test_fade_in_wrapper() {
        use ratatui::widgets::Block;

        let block = Block::default();
        let fade = FadeIn::new(block, 500);

        assert!(!fade.is_complete());
        // Progress should be near 0 at creation
        assert!(
            fade.progress() < 0.01,
            "Progress should be near 0 at creation"
        );
    }

    #[test]
    fn test_fade_in_reset() {
        use ratatui::widgets::Block;

        let block = Block::default();
        let mut fade = FadeIn::new(block, 100);

        // Wait for some progress
        std::thread::sleep(Duration::from_millis(50));
        let progress_before = fade.progress();
        assert!(progress_before > 0.0, "Progress should have increased");

        fade.reset();
        assert!(!fade.is_complete());
        // Progress should be near 0 after reset
        assert!(
            fade.progress() < 0.01,
            "Progress should be near 0 after reset"
        );
    }

    #[test]
    fn test_opacity_wrapper() {
        use ratatui::widgets::Block;

        let block = Block::default();
        let opacity = Opacity::new(block, 0.5);

        assert_eq!(opacity.get_opacity(), 0.5);
    }

    #[test]
    fn test_opacity_clamping() {
        use ratatui::widgets::Block;

        let block = Block::default();

        let opacity_high = Opacity::new(block.clone(), 1.5);
        assert_eq!(opacity_high.get_opacity(), 1.0);

        let opacity_low = Opacity::new(block, -0.5);
        assert_eq!(opacity_low.get_opacity(), 0.0);
    }

    #[test]
    fn test_opacity_builder() {
        use ratatui::widgets::Block;

        let block = Block::default();
        let opacity = Opacity::new(block, 0.5).opacity(0.75);

        assert_eq!(opacity.get_opacity(), 0.75);
    }

    #[test]
    fn test_animated_widget_render() {
        use ratatui::widgets::Block;

        let block = Block::default();
        let animation = AnimationController::fade_in(500);
        let animated = AnimatedWidget::new(block, animation);

        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 10));
        animated.render(Rect::new(0, 0, 10, 10), &mut buf);

        // Should render without panic
    }

    #[test]
    fn test_fade_in_render() {
        use ratatui::widgets::Block;

        let block = Block::default();
        let fade = FadeIn::new(block, 500);

        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 10));
        fade.render(Rect::new(0, 0, 10, 10), &mut buf);

        // Should render without panic
    }

    #[test]
    fn test_opacity_render() {
        use ratatui::widgets::Paragraph;

        let paragraph = Paragraph::new("Hello");
        let opacity = Opacity::new(paragraph, 0.5);

        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 10));
        opacity.render(Rect::new(0, 0, 10, 10), &mut buf);

        // Should render without panic
    }
}
