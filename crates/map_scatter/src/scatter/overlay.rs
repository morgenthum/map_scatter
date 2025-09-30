//! Overlay textures and utilities for building mask textures from placements.
use glam::Vec2;

use crate::fieldgraph::{Texture, TextureChannel};

/// A 2D overlay texture with a single red channel.
#[derive(Clone)]
pub struct OverlayTexture {
    pub domain_extent: Vec2,
    pub width: u32,
    pub height: u32,
    pub data_r: Vec<f32>,
}

impl OverlayTexture {
    /// Create a new [`OverlayTexture`].
    pub fn new(domain_extent: Vec2, width: u32, height: u32, data_r: Vec<f32>) -> Self {
        Self {
            domain_extent,
            width,
            height,
            data_r,
        }
    }

    /// Sample the texture at a position in domain space.
    pub fn sample_domain(&self, channel: TextureChannel, p: Vec2) -> f32 {
        if self.width == 0 || self.height == 0 {
            return if matches!(channel, TextureChannel::A) {
                1.0
            } else {
                0.0
            };
        }

        let u = if self.domain_extent.x != 0.0 {
            ((p.x / self.domain_extent.x) + 0.5).clamp(0.0, 1.0)
        } else {
            0.5
        };
        let v = if self.domain_extent.y != 0.0 {
            ((p.y / self.domain_extent.y) + 0.5).clamp(0.0, 1.0)
        } else {
            0.5
        };

        let w1 = self.width - 1;
        let h1 = self.height - 1;
        let x = ((u * self.width as f32) as u32).min(w1);
        let y = ((v * self.height as f32) as u32).min(h1);
        let idx = (y as usize) * (self.width as usize) + (x as usize);

        match channel {
            TextureChannel::R => self.data_r.get(idx).copied().unwrap_or(0.0),
            TextureChannel::A => 1.0,
            _ => 0.0,
        }
    }
}

impl Texture for OverlayTexture {
    fn sample(&self, channel: TextureChannel, p: Vec2) -> f32 {
        self.sample_domain(channel, p)
    }
}

pub fn build_overlay_mask_from_positions(
    domain_extent: Vec2,
    positions: &[Vec2],
    width: u32,
    height: u32,
    stamp_radius_px: i32,
) -> OverlayTexture {
    build_overlay_mask_from_positions_with_shape(
        domain_extent,
        positions,
        width,
        height,
        stamp_radius_px,
    )
}

pub fn build_overlay_mask_from_positions_with_shape(
    domain_extent: Vec2,
    positions: &[Vec2],
    width: u32,
    height: u32,
    stamp_radius_px: i32,
) -> OverlayTexture {
    let len = (width as usize) * (height as usize);
    if len == 0 {
        return OverlayTexture::new(domain_extent, width, height, Vec::new());
    }
    let mut data = vec![0.0f32; len];
    let w_i = width as i32;
    let h_i = height as i32;

    for &position in positions {
        let u = if domain_extent.x != 0.0 {
            ((position.x / domain_extent.x) + 0.5).clamp(0.0, 1.0)
        } else {
            0.5
        };
        let v = if domain_extent.y != 0.0 {
            ((position.y / domain_extent.y) + 0.5).clamp(0.0, 1.0)
        } else {
            0.5
        };

        let px = ((u * width as f32).floor() as i32).clamp(0, w_i - 1);
        let py = ((v * height as f32).floor() as i32).clamp(0, h_i - 1);

        let start_x = (px - stamp_radius_px).max(0);
        let end_x = (px + stamp_radius_px).min(w_i - 1);
        let start_y = (py - stamp_radius_px).max(0);
        let end_y = (py + stamp_radius_px).min(h_i - 1);

        let r2 = stamp_radius_px * stamp_radius_px;

        for sy in start_y..=end_y {
            let row = (sy as usize) * (width as usize);
            for sx in start_x..=end_x {
                let idx = row + sx as usize;

                let stamp = {
                    let dx = sx - px;
                    let dy = sy - py;
                    dx * dx + dy * dy <= r2
                };

                if stamp {
                    data[idx] = 1.0;
                }
            }
        }
    }

    OverlayTexture::new(domain_extent, width, height, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_domain_handles_empty_texture() {
        let overlay = OverlayTexture::new(Vec2::ZERO, 0, 0, Vec::new());
        assert_eq!(overlay.sample_domain(TextureChannel::R, Vec2::ZERO), 0.0);
        assert_eq!(overlay.sample_domain(TextureChannel::A, Vec2::ZERO), 1.0);
    }

    #[test]
    fn sample_domain_reads_r_channel() {
        let overlay = OverlayTexture::new(Vec2::new(2.0, 2.0), 2, 2, vec![0.0, 0.5, 0.75, 1.0]);
        assert_eq!(
            overlay.sample_domain(TextureChannel::R, Vec2::new(-1.0, -1.0)),
            0.0
        );
        assert_eq!(
            overlay.sample_domain(TextureChannel::R, Vec2::new(0.99, 0.99)),
            1.0
        );
        assert_eq!(
            overlay.sample_domain(TextureChannel::A, Vec2::new(0.0, 0.0)),
            1.0
        );
        assert_eq!(
            overlay.sample_domain(TextureChannel::G, Vec2::new(0.0, 0.0)),
            0.0
        );
    }

    #[test]
    fn build_overlay_mask_sets_pixels() {
        let texture =
            build_overlay_mask_from_positions(Vec2::new(2.0, 2.0), &[Vec2::ZERO], 2, 2, 0);
        assert_eq!(texture.data_r.iter().filter(|v| **v > 0.0).count(), 1);
    }
}
