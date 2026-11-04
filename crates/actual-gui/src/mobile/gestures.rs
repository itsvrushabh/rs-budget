//! Touch gesture recognition and classifier for mobile clients.
//!
//! Recognizes Tap, DoubleTap, LongPress, horizontal Swipe (e.g. swipe-to-delete),
//! and vertical Pull-to-Refresh sync gestures.

use serde::{Deserialize, Serialize};

/// 2D touch coordinate in logical screen points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TouchPoint {
    pub x: f32,
    pub y: f32,
    pub timestamp_ms: u64,
}

/// Raw touch lifecycle event.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TouchPhase {
    Began,
    Moved,
    Ended,
    Cancelled,
}

/// Classified touch gesture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Gesture {
    Tap { x: f32, y: f32 },
    DoubleTap { x: f32, y: f32 },
    LongPress { x: f32, y: f32, duration_ms: u64 },
    SwipeLeft { distance: f32, velocity: f32 },
    SwipeRight { distance: f32, velocity: f32 },
    PullToRefresh { distance_y: f32 },
}

/// Gesture state machine classifying raw touches into high-level gestures.
#[derive(Debug, Clone)]
pub struct GestureClassifier {
    start_point: Option<TouchPoint>,
    last_point: Option<TouchPoint>,
    last_tap_timestamp_ms: Option<u64>,
    tap_distance_threshold: f32,
    swipe_min_distance: f32,
    pull_to_refresh_threshold: f32,
    long_press_duration_ms: u64,
}

impl Default for GestureClassifier {
    fn default() -> Self {
        Self {
            start_point: None,
            last_point: None,
            last_tap_timestamp_ms: None,
            tap_distance_threshold: 15.0,    // pt
            swipe_min_distance: 60.0,        // pt
            pull_to_refresh_threshold: 80.0, // pt
            long_press_duration_ms: 500,     // ms
        }
    }
}

impl GestureClassifier {
    /// Feed a raw touch event into the classifier, returning any recognized gesture.
    pub fn process_touch(
        &mut self,
        phase: TouchPhase,
        x: f32,
        y: f32,
        timestamp_ms: u64,
    ) -> Option<Gesture> {
        let current = TouchPoint { x, y, timestamp_ms };

        match phase {
            TouchPhase::Began => {
                self.start_point = Some(current);
                self.last_point = Some(current);
                None
            }
            TouchPhase::Moved => {
                self.last_point = Some(current);
                if let Some(start) = self.start_point {
                    let dy = y - start.y;
                    let dx = (x - start.x).abs();
                    // Pull to refresh if vertical drag dominates and crosses threshold
                    if dy > self.pull_to_refresh_threshold && dx < (dy * 0.5) {
                        return Some(Gesture::PullToRefresh { distance_y: dy });
                    }
                }
                None
            }
            TouchPhase::Ended => {
                let start = self.start_point.take()?;
                self.last_point = None;

                let dx = x - start.x;
                let dy = y - start.y;
                let dist = (dx * dx + dy * dy).sqrt();
                let dt = (timestamp_ms.saturating_sub(start.timestamp_ms)).max(1) as f32; // ms
                let velocity = (dist / dt) * 1000.0; // pt/s

                // Check Swipe gestures
                if dist >= self.swipe_min_distance && dx.abs() > (dy.abs() * 1.5) {
                    if dx < 0.0 {
                        return Some(Gesture::SwipeLeft {
                            distance: dx.abs(),
                            velocity,
                        });
                    } else {
                        return Some(Gesture::SwipeRight {
                            distance: dx,
                            velocity,
                        });
                    }
                }

                // Check Tap / DoubleTap / LongPress
                if dist <= self.tap_distance_threshold {
                    let duration = timestamp_ms.saturating_sub(start.timestamp_ms);
                    if duration >= self.long_press_duration_ms {
                        return Some(Gesture::LongPress {
                            x,
                            y,
                            duration_ms: duration,
                        });
                    }

                    // Check DoubleTap
                    if let Some(prev_tap) = self.last_tap_timestamp_ms {
                        if timestamp_ms.saturating_sub(prev_tap) <= 350 {
                            self.last_tap_timestamp_ms = None;
                            return Some(Gesture::DoubleTap { x, y });
                        }
                    }

                    self.last_tap_timestamp_ms = Some(timestamp_ms);
                    return Some(Gesture::Tap { x, y });
                }

                None
            }
            TouchPhase::Cancelled => {
                self.start_point = None;
                self.last_point = None;
                None
            }
        }
    }
}
