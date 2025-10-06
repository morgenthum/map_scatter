use std::sync::Arc;

use bevy::prelude::Image;
use bevy::render::render_resource::TextureFormat;
use glam::Vec2;
use map_scatter::prelude::{Texture, TextureChannel};

/// CPU-side adapter that snapshots a Bevy [`Image`] and implements [`Texture`].
/// This copies the pixel data into memory. Re-create the [`ImageTexture`] when the source
///   [`Image`] changes.
pub struct ImageTexture {
    domain_extent: Vec2,
    format: TextureFormat,
    pixels: Arc<Vec<u8>>,
    width: u32,
    height: u32,
}

impl ImageTexture {
    /// Creates an [`ImageTexture`] snapshot from a Bevy [`Image`] and maps it to a specified domain extent.
    pub fn from_image(image: &Image, domain_extent: Vec2) -> Option<Self> {
        let format = image.texture_descriptor.format;

        let supported = matches!(
            format,
            TextureFormat::R8Unorm
                | TextureFormat::Rgba8Unorm
                | TextureFormat::Rgba8UnormSrgb
                | TextureFormat::Bgra8Unorm
                | TextureFormat::Bgra8UnormSrgb
        );

        if !supported {
            return None;
        }

        let pixels = Arc::new(image.data.clone().unwrap());
        let width = image.texture_descriptor.size.width;
        let height = image.texture_descriptor.size.height;

        Some(Self {
            domain_extent,
            format,
            pixels,
            width,
            height,
        })
    }

    #[inline]
    fn bytes_per_pixel(&self) -> usize {
        match self.format {
            TextureFormat::R8Unorm => 1,
            TextureFormat::Rgba8Unorm
            | TextureFormat::Rgba8UnormSrgb
            | TextureFormat::Bgra8Unorm
            | TextureFormat::Bgra8UnormSrgb => 4,
            _ => 0,
        }
    }

    #[inline]
    fn channel_offset(&self, channel: TextureChannel) -> Option<usize> {
        match self.format {
            TextureFormat::R8Unorm => match channel {
                TextureChannel::R => Some(0),
                _ => None,
            },
            TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb => match channel {
                TextureChannel::R => Some(0),
                TextureChannel::G => Some(1),
                TextureChannel::B => Some(2),
                TextureChannel::A => Some(3),
            },
            TextureFormat::Bgra8Unorm | TextureFormat::Bgra8UnormSrgb => match channel {
                TextureChannel::B => Some(0),
                TextureChannel::G => Some(1),
                TextureChannel::R => Some(2),
                TextureChannel::A => Some(3),
            },
            _ => None,
        }
    }
}

impl Texture for ImageTexture {
    fn sample(&self, channel: TextureChannel, p: Vec2) -> f32 {
        let bpp = self.bytes_per_pixel();
        if bpp == 0 {
            return 0.0;
        }

        // Map world/domain coordinates to image texels using a centered domain, like overlays.
        // Use a configurable domain extent: x∈[-dw/2,dw/2], y∈[-dh/2,dh/2], independent of image size.
        let (w, h) = (self.width, self.height);
        if w == 0 || h == 0 {
            return 0.0;
        }
        let (dw, dh) = (self.domain_extent.x, self.domain_extent.y);
        if dw == 0.0 || dh == 0.0 {
            return 0.0;
        }
        let u = ((p.x / dw) + 0.5).clamp(0.0, 1.0);
        let v = ((p.y / dh) + 0.5).clamp(0.0, 1.0);
        let x = ((u * w as f32) as u32).min(w.saturating_sub(1));
        let y = ((v * h as f32) as u32).min(h.saturating_sub(1));

        let idx = (y as usize) * (self.width as usize) + (x as usize);
        let base = idx * bpp;

        let Some(co) = self.channel_offset(channel) else {
            return 0.0;
        };

        let byte = self.pixels.get(base + co).copied().unwrap_or(0);
        (byte as f32) / 255.0
    }
}
