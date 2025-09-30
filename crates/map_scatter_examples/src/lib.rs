#![forbid(unsafe_code)]
//! Helpers for running and rendering `map_scatter` examples.
//!
//! This crate keeps all example utilities colocated with the example binaries, so the
//! published `map_scatter` crate stays small and dependency-free.
//!
//! Typical usage in example bins:
//!   - bring the helpers into scope with:
//!       use map_scatter_examples::rendering::{ KindStyle, RenderConfig, render_run_result_to_png, init_tracing };
//!   - configure styles and call `render_run_result_to_png` with the placements.

pub mod rendering {
    use std::collections::HashMap;
    use std::path::Path;

    use glam::Vec2;
    use image::{DynamicImage, ImageError, ImageReader, RgbImage};
    use map_scatter::fieldgraph::{Texture, TextureChannel, TextureRegistry};
    use map_scatter::prelude::RunResult;

    /// Texture loaded from a PNG, with world-space mapping and RGBA storage.
    #[derive(Clone, Debug)]
    pub struct PngTexture {
        pub origin: Vec2,
        pub extent: Vec2,
        pub width: u32,
        pub height: u32,
        pub data_rgba: Vec<[f32; 4]>,
    }

    impl PngTexture {
        pub fn from_path(
            path: impl AsRef<Path>,
            origin: Vec2,
            extent: Vec2,
        ) -> Result<Self, ImageError> {
            let img = ImageReader::open(path)?.decode()?;
            Ok(Self::from_dynamic(img, origin, extent))
        }

        pub fn from_dynamic(img: DynamicImage, origin: Vec2, extent: Vec2) -> Self {
            let rgba = img.to_rgba8();
            let (width, height) = (rgba.width(), rgba.height());

            let mut data = Vec::with_capacity((width as usize) * (height as usize));
            for y in 0..height {
                for x in 0..width {
                    let p = rgba.get_pixel(x, y).0;
                    data.push([
                        (p[0] as f32) / 255.0,
                        (p[1] as f32) / 255.0,
                        (p[2] as f32) / 255.0,
                        (p[3] as f32) / 255.0,
                    ]);
                }
            }

            Self {
                origin,
                extent,
                width,
                height,
                data_rgba: data,
            }
        }

        #[inline]
        pub fn sample(&self, channel: TextureChannel, p: Vec2) -> f32 {
            if self.width == 0 || self.height == 0 {
                return 0.0;
            }

            let u = if self.extent.x != 0.0 {
                ((p.x - self.origin.x) / self.extent.x).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let v = if self.extent.y != 0.0 {
                ((p.y - self.origin.y) / self.extent.y).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let x = (u * self.width as f32)
                .floor()
                .clamp(0.0, (self.width - 1) as f32) as u32;
            let y = (v * self.height as f32)
                .floor()
                .clamp(0.0, (self.height - 1) as f32) as u32;

            let idx = (y as usize) * (self.width as usize) + (x as usize);
            let px = self.data_rgba[idx];

            match channel {
                TextureChannel::R => px[0],
                TextureChannel::G => px[1],
                TextureChannel::B => px[2],
                TextureChannel::A => px[3],
            }
        }
    }

    impl Texture for PngTexture {
        #[inline]
        fn sample(&self, channel: TextureChannel, p: Vec2) -> f32 {
            PngTexture::sample(self, channel, p)
        }
    }

    /// Simple collection for loading and registering PNG textures by id.
    #[derive(Default)]
    pub struct PngTextures {
        by_id: HashMap<String, PngTexture>,
    }

    impl PngTextures {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn insert(&mut self, id: impl Into<String>, tex: PngTexture) {
            self.by_id.insert(id.into(), tex);
        }

        pub fn load_png(
            &mut self,
            id: impl Into<String>,
            path: impl AsRef<Path>,
            origin: Vec2,
            extent: Vec2,
        ) -> Result<(), ImageError> {
            let id = id.into();
            let tex = PngTexture::from_path(path, origin, extent)?;
            self.by_id.insert(id, tex);
            Ok(())
        }

        pub fn get(&self, id: &str) -> Option<&PngTexture> {
            self.by_id.get(id)
        }

        pub fn register_all_into(&self, registry: &mut TextureRegistry) {
            for (id, tex) in &self.by_id {
                registry.register(id.clone(), tex.clone());
            }
        }
    }

    /// How to visualize placements of a given kind.
    #[derive(Clone, Debug)]
    pub enum KindStyle {
        /// Solid disk (radius in pixels) with an RGB color.
        Circle { color: [u8; 3], radius: i32 },
        /// Sprite blit (centered) using a PNG by id and a scale factor.
        Sprite { sprite_id: String, scale: f32 },
    }

    /// Rendering configuration shared by the example PNG renderers.
    #[derive(Clone, Debug)]
    pub struct RenderConfig {
        pub image_size: (u32, u32),
        pub domain_extent: Vec2,
        pub background: [u8; 3],
        pub styles: HashMap<String, KindStyle>,
        pub sprites: HashMap<String, PngTexture>,
    }

    impl RenderConfig {
        pub fn new(image_size: (u32, u32), domain_extent: Vec2) -> Self {
            Self {
                image_size,
                domain_extent,
                background: [230, 230, 230],
                styles: HashMap::new(),
                sprites: HashMap::new(),
            }
        }

        pub fn with_background(mut self, bg: [u8; 3]) -> Self {
            self.background = bg;
            self
        }

        pub fn set_kind_style(&mut self, id: impl Into<String>, style: KindStyle) -> &mut Self {
            self.styles.insert(id.into(), style);
            self
        }

        pub fn add_sprite(&mut self, id: impl Into<String>, tex: PngTexture) -> &mut Self {
            self.sprites.insert(id.into(), tex);
            self
        }

        pub fn load_sprite_png(
            &mut self,
            id: impl Into<String>,
            path: impl AsRef<Path>,
        ) -> Result<&mut Self, ImageError> {
            let img = ImageReader::open(path)?.decode()?;
            let tex = PngTexture::from_dynamic(img, Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0));
            self.sprites.insert(id.into(), tex);
            Ok(self)
        }

        pub fn style_for(&self, kind_id: &str) -> KindStyle {
            self.styles
                .get(kind_id)
                .cloned()
                .unwrap_or(KindStyle::Circle {
                    color: [0, 0, 0],
                    radius: 1,
                })
        }
    }

    /// Initialize logging/tracing for examples.
    /// This is a no-op placeholder to avoid adding extra dependencies to the examples crate.
    /// Examples can still call this safely.
    #[inline]
    pub fn init_tracing() {
        // Intentionally left blank.
        // You can swap this for a 'tracing_subscriber' or 'env_logger' init if preferred.
    }

    /// Render a [`RunResult`] into a PNG file using the provided [`RenderConfig`].
    pub fn render_run_result_to_png(
        result: &RunResult,
        cfg: &RenderConfig,
        out_path: impl AsRef<Path>,
    ) -> image::ImageResult<()> {
        let out_path = out_path.as_ref();
        let (w, h) = cfg.image_size;

        // Console summary (keep noise low and library-free)
        let mut per_kind: HashMap<String, usize> = HashMap::new();
        for p in &result.placements {
            *per_kind.entry(p.kind_id.clone()).or_insert(0) += 1;
        }
        let mut kind_parts: Vec<String> = per_kind
            .iter()
            .map(|(k, n)| format!("{}: {}", k, n))
            .collect();
        kind_parts.sort();

        println!(
            "[map_scatter_examples] placements={}, evaluated={}, rejected={}",
            result.placements.len(),
            result.positions_evaluated,
            result.positions_rejected
        );
        println!(
            "[map_scatter_examples] image={}x{}, domain=({}, {})",
            w, h, cfg.domain_extent.x, cfg.domain_extent.y
        );
        println!("[map_scatter_examples] output: {}", out_path.display());
        if !kind_parts.is_empty() {
            println!("[map_scatter_examples] per-kind: {}", kind_parts.join(", "));
        }

        let mut img: RgbImage =
            image::ImageBuffer::from_fn(w, h, |_x, _y| image::Rgb(cfg.background));

        for p in &result.placements {
            let style = cfg.style_for(&p.kind_id);
            let (px, py) = world_to_pixel(p.position, cfg.domain_extent, w, h);
            match style {
                KindStyle::Circle { color, radius } => {
                    draw_disc(&mut img, px as i32, py as i32, radius, image::Rgb(color));
                }
                KindStyle::Sprite { sprite_id, scale } => {
                    if let Some(sprite) = cfg.sprites.get(&sprite_id) {
                        blit_sprite(&mut img, px as i32, py as i32, sprite, scale);
                    }
                }
            }
        }

        img.save(out_path)?;
        println!("[map_scatter_examples] saved PNG: {}", out_path.display());
        Ok(())
    }

    fn world_to_pixel(p: Vec2, domain: Vec2, w: u32, h: u32) -> (u32, u32) {
        let u = ((p.x / domain.x) + 0.5).clamp(0.0, 1.0);
        let v = ((p.y / domain.y) + 0.5).clamp(0.0, 1.0);
        let x = (u * (w as f32)).clamp(0.0, (w - 1) as f32) as u32;
        let y = (v * (h as f32)).clamp(0.0, (h - 1) as f32) as u32;
        (x, y)
    }

    fn draw_disc(img: &mut RgbImage, cx: i32, cy: i32, r: i32, color: image::Rgb<u8>) {
        let w = img.width() as i32;
        let h = img.height() as i32;
        let r2 = r * r;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r2 {
                    let x = cx + dx;
                    let y = cy + dy;
                    if x >= 0 && x < w && y >= 0 && y < h {
                        img.put_pixel(x as u32, y as u32, color);
                    }
                }
            }
        }
    }

    fn blit_sprite(img: &mut RgbImage, cx: i32, cy: i32, sprite: &PngTexture, scale: f32) {
        if sprite.width == 0 || sprite.height == 0 {
            return;
        }

        let scale = scale.max(0.01);
        let sw = ((sprite.width as f32) * scale).round().max(1.0) as u32;
        let sh = ((sprite.height as f32) * scale).round().max(1.0) as u32;

        let left = cx - (sw as i32) / 2;
        let top = cy - (sh as i32) / 2;

        let img_w = img.width() as i32;
        let img_h = img.height() as i32;

        for dy in 0..(sh as i32) {
            let dest_y = top + dy;
            if dest_y < 0 || dest_y >= img_h {
                continue;
            }
            let v = dy as f32 / (sh as f32);
            let sy = (v * (sprite.height as f32))
                .floor()
                .clamp(0.0, (sprite.height - 1) as f32) as u32;

            for dx in 0..(sw as i32) {
                let dest_x = left + dx;
                if dest_x < 0 || dest_x >= img_w {
                    continue;
                }

                let u = dx as f32 / (sw as f32);
                let sx = (u * (sprite.width as f32))
                    .floor()
                    .clamp(0.0, (sprite.width - 1) as f32) as u32;

                let idx = (sy as usize) * (sprite.width as usize) + (sx as usize);
                let px = sprite.data_rgba[idx];
                let sr = (px[0] * 255.0).round() as u8;
                let sg = (px[1] * 255.0).round() as u8;
                let sb = (px[2] * 255.0).round() as u8;
                let sa = px[3].clamp(0.0, 1.0);

                if sa <= 0.0 {
                    continue;
                }

                let dst = img.get_pixel_mut(dest_x as u32, dest_y as u32);
                let dr = dst[0] as f32;
                let dg = dst[1] as f32;
                let db = dst[2] as f32;

                let out_r = (sa * (sr as f32) + (1.0 - sa) * dr)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                let out_g = (sa * (sg as f32) + (1.0 - sa) * dg)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                let out_b = (sa * (sb as f32) + (1.0 - sa) * db)
                    .round()
                    .clamp(0.0, 255.0) as u8;

                *dst = image::Rgb([out_r, out_g, out_b]);
            }
        }
    }
}

pub use rendering::{
    init_tracing, render_run_result_to_png, KindStyle, PngTexture, PngTextures, RenderConfig,
};
