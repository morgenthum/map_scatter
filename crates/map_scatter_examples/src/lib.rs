#![forbid(unsafe_code)]

mod rendering;

pub use rendering::{
    init_tracing, render_run_result_to_png, KindStyle, PngTexture, PngTextures, RenderConfig,
};
