use std::time::{Duration, Instant};

use crate::{capture::platform::windows::CapturedFrame, errors::IpcError};

use super::{
    CursorSnapshot,
    raster::{
        self, DamageRect, Ring, cursor_bounds, draw_pointer, draw_ring, rect_is_empty, union_rect,
    },
};

const CLICK_ANIMATION: Duration = Duration::from_millis(360);

#[derive(Debug, Clone, Copy)]
struct ClickEffect {
    x: i32,
    y: i32,
    started_at: Instant,
}

/// Composes the pointer in a reusable output buffer and saves only its small damage patch.
#[derive(Default)]
pub(crate) struct CursorCompositor {
    width: u32,
    height: u32,
    pixels_bgra: Vec<u8>,
    restore_rect: Option<DamageRect>,
    restore_pixels: Vec<u8>,
    position: Option<(i32, i32)>,
    left_down: bool,
    right_down: bool,
    drag_started_at: Option<Instant>,
    click_effect: Option<ClickEffect>,
    animation_was_active: bool,
    pending_base: bool,
}

impl CursorCompositor {
    pub(crate) fn set_base_frame(&mut self, frame: &CapturedFrame) -> Result<(), IpcError> {
        let expected = usize::try_from(frame.width)
            .ok()
            .and_then(|width| {
                usize::try_from(frame.height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixel_count| pixel_count.checked_mul(4))
            .ok_or_else(|| {
                IpcError::new(
                    "frame_layout_invalid",
                    "The captured frame is too large for cursor composition.",
                    true,
                )
            })?;
        if frame.width < 2 || frame.height < 2 || frame.pixels_bgra.len() != expected {
            return Err(IpcError::new(
                "frame_layout_invalid",
                "The captured frame has an invalid BGRA layout for cursor composition.",
                true,
            ));
        }

        self.width = frame.width;
        self.height = frame.height;
        self.pixels_bgra.clone_from(&frame.pixels_bgra);
        self.restore_rect = None;
        self.restore_pixels.clear();
        self.pending_base = true;
        Ok(())
    }

    /// Returns true when the composed output changed and should be encoded at the next frame tick.
    pub(crate) fn update(&mut self, snapshot: Option<CursorSnapshot>, now: Instant) -> bool {
        let mut input_changed = false;
        if let Some(snapshot) = snapshot {
            let next_position = Some((snapshot.x, snapshot.y));
            input_changed |= self.position != next_position
                || self.left_down != snapshot.left_down
                || self.right_down != snapshot.right_down;

            if snapshot.left_down && !self.left_down {
                self.drag_started_at = Some(now);
                self.click_effect = Some(ClickEffect {
                    x: snapshot.x,
                    y: snapshot.y,
                    started_at: now,
                });
            } else if !snapshot.left_down && self.left_down {
                if self.drag_started_at.is_some_and(|started| {
                    now.duration_since(started) >= Duration::from_millis(120)
                }) {
                    self.click_effect = Some(ClickEffect {
                        x: snapshot.x,
                        y: snapshot.y,
                        started_at: now,
                    });
                }
                self.drag_started_at = None;
            }
            if snapshot.right_down && !self.right_down {
                self.click_effect = Some(ClickEffect {
                    x: snapshot.x,
                    y: snapshot.y,
                    started_at: now,
                });
            }

            self.position = next_position;
            self.left_down = snapshot.left_down;
            self.right_down = snapshot.right_down;
        } else {
            input_changed |= self.position.take().is_some() || self.left_down || self.right_down;
            self.left_down = false;
            self.right_down = false;
            self.drag_started_at = None;
        }

        let click_active = self
            .click_effect
            .is_some_and(|effect| now.duration_since(effect.started_at) < CLICK_ANIMATION);
        if !click_active {
            self.click_effect = None;
        }
        let animation_active = click_active || self.left_down;
        let should_render =
            self.pending_base || input_changed || animation_active || self.animation_was_active;
        if !should_render {
            return false;
        }

        self.restore_previous_overlay();
        let mut new_damage = self
            .position
            .map(|(x, y)| cursor_bounds(x, y, self.width, self.height));
        if click_active && let Some(effect) = self.click_effect {
            new_damage = union_rect(
                new_damage,
                cursor_bounds(
                    effect.x.saturating_add(9),
                    effect.y.saturating_add(10),
                    self.width,
                    self.height,
                ),
            );
        }

        if let Some(damage) = new_damage.filter(|rect| !rect_is_empty(*rect)) {
            self.save_restore_patch(damage);
            self.draw_effects(click_active, now);
            if let Some((x, y)) = self.position {
                draw_pointer(&mut self.pixels_bgra, self.width, self.height, x, y);
            }
            self.restore_rect = Some(damage);
        }

        self.animation_was_active = animation_active;
        self.pending_base = false;
        true
    }

    pub(crate) fn pixels(&self) -> &[u8] {
        &self.pixels_bgra
    }

    pub(crate) fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels_bgra
    }

    fn draw_effects(&mut self, click_active: bool, now: Instant) {
        if click_active && let Some(effect) = self.click_effect {
            let progress =
                now.duration_since(effect.started_at).as_secs_f32() / CLICK_ANIMATION.as_secs_f32();
            let eased = 1.0 - (1.0 - progress.clamp(0.0, 1.0)).powi(2);
            let radius = 8.0 + 24.0 * eased;
            let alpha = ((1.0 - eased) * 190.0).round() as u8;
            draw_ring(
                &mut self.pixels_bgra,
                self.width,
                self.height,
                Ring {
                    center_x: effect.x.saturating_add(9),
                    center_y: effect.y.saturating_add(10),
                    radius: radius + 1.5,
                    thickness: 3.0,
                    color: [255, 255, 255, 255],
                    alpha,
                },
            );
            draw_ring(
                &mut self.pixels_bgra,
                self.width,
                self.height,
                Ring {
                    center_x: effect.x.saturating_add(9),
                    center_y: effect.y.saturating_add(10),
                    radius,
                    thickness: 2.0,
                    color: [0, 0, 0, 255],
                    alpha,
                },
            );
        }

        if self.left_down
            && let Some((x, y)) = self.position
        {
            let elapsed = self
                .drag_started_at
                .map_or(0.0, |started| now.duration_since(started).as_secs_f32());
            let radius = 12.0 + (elapsed * 7.0).sin() * 2.0;
            draw_ring(
                &mut self.pixels_bgra,
                self.width,
                self.height,
                Ring {
                    center_x: x.saturating_add(9),
                    center_y: y.saturating_add(10),
                    radius: radius + 1.5,
                    thickness: 4.0,
                    color: [255, 255, 255, 255],
                    alpha: 230,
                },
            );
            draw_ring(
                &mut self.pixels_bgra,
                self.width,
                self.height,
                Ring {
                    center_x: x.saturating_add(9),
                    center_y: y.saturating_add(10),
                    radius,
                    thickness: 2.0,
                    color: [0, 0, 0, 255],
                    alpha: 235,
                },
            );
        }
    }

    fn restore_previous_overlay(&mut self) {
        let Some(rect) = self.restore_rect.take() else {
            return;
        };
        let mut patch_offset = 0;
        for y in rect.top..rect.bottom {
            let start = raster::pixel_offset(y, rect.left, self.width);
            let end = raster::pixel_offset(y, rect.right, self.width);
            let row_length = end - start;
            self.pixels_bgra[start..end]
                .copy_from_slice(&self.restore_pixels[patch_offset..patch_offset + row_length]);
            patch_offset += row_length;
        }
        self.restore_pixels.clear();
    }

    fn save_restore_patch(&mut self, rect: DamageRect) {
        self.restore_pixels.clear();
        for y in rect.top..rect.bottom {
            let start = raster::pixel_offset(y, rect.left, self.width);
            let end = raster::pixel_offset(y, rect.right, self.width);
            self.restore_pixels
                .extend_from_slice(&self.pixels_bgra[start..end]);
        }
    }
}
