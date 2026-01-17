//! Euclidean Distance Transform (EDT) utilities for the field graph runtime.
//!
//! Computes EDT rasters from thresholded fields and normalizes distances.
//!
//! This implementation is based on the Felzenszwalb-Huttenlocher algorithm,
//! which computes exact Euclidean distances using a separable approach with
//! two 1D passes.
use crate::fieldgraph::runtime::FieldRuntime;
use crate::fieldgraph::{ChunkGrid, ChunkId, Raster};

/// Computes the Euclidean Distance Transform (EDT) of a binary mask derived from the input field,
/// then normalizes the distances.
pub fn bake_edt_normalize_params(
    runtime: &mut FieldRuntime<'_>,
    input_field: &str,
    threshold: f32,
    d_max: f32,
    chunk: ChunkId,
    grid: &ChunkGrid,
) -> Raster {
    let (tw, th) = (grid.total_width(), grid.total_height());
    let mut mask: Vec<u8> = vec![0; tw * th];

    // Create binary mask from input field
    for iy in 0..th as isize {
        for ix in 0..tw as isize {
            let p = grid.index_to_world(ix, iy);
            let v = runtime.sample(input_field, p, chunk, grid);
            let idx = (iy as usize) * tw + ix as usize;
            mask[idx] = if v >= threshold { 1 } else { 0 };
        }
    }

    // Compute EDT
    let edt = edt_unsigned(&mask, tw, th);

    // Create normalized raster
    let mut raster = Raster::new(grid.clone());
    if d_max > 0.0 {
        for (i, val) in edt.iter().enumerate() {
            raster.data[i] = (*val / d_max).min(1.0);
        }
    } else {
        for (i, val) in edt.iter().enumerate() {
            raster.data[i] = (*val).min(1.0);
        }
    }
    raster
}

/// Computes the 1D Euclidean Distance Transform using the lower envelope algorithm.
fn edt_1d(f: &[f32], output: &mut [f32]) {
    let n = f.len();
    if n == 0 {
        return;
    }

    debug_assert_eq!(
        f.len(),
        output.len(),
        "Input and output must have same length"
    );

    let mut v = vec![0; n];
    let mut z = vec![0.0; n + 1];
    let mut k = 0;

    // Initialize first parabola
    v[0] = 0;
    z[0] = f32::NEG_INFINITY;
    z[1] = f32::INFINITY;

    // Compute lower envelope
    for q in 1..n {
        loop {
            if k == 0 {
                let s = intersection_safe(q, v[0], f);
                if s <= z[0] {
                    break;
                }
            }

            let r = v[k];
            let s = intersection_safe(q, r, f);

            if s <= z[k] {
                if k > 0 {
                    k -= 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Add new parabola
        k += 1;
        debug_assert!(k < v.len(), "k should not exceed v bounds");
        v[k] = q;

        if k > 0 {
            z[k] = intersection_safe(q, v[k - 1], f);
        }
        z[k + 1] = f32::INFINITY;
    }

    // Fill output with minimum values
    k = 0;
    for (q, dq) in output.iter_mut().enumerate() {
        // Find the parabola that gives minimum at position q
        while k + 1 < z.len() && z[k + 1] < q as f32 {
            k += 1;
        }

        // Safety: ensure k is within bounds
        debug_assert!(k < v.len(), "k should be within v bounds");

        let dx = (q as f32) - (v[k] as f32);
        *dq = dx * dx + f[v[k]];
    }
}

/// Computes the intersection point of two parabolas in the lower envelope.
fn intersection_safe(i: usize, j: usize, f: &[f32]) -> f32 {
    debug_assert!(i < f.len() && j < f.len(), "Indices must be within bounds");

    if i == j {
        // Same parabola - no intersection
        return f32::INFINITY;
    }

    let fi = f[i];
    let fj = f[j];

    // Check for invalid values
    if !fi.is_finite() || !fj.is_finite() {
        return f32::INFINITY;
    }

    let numerator = (fi + (i * i) as f32) - (fj + (j * j) as f32);
    let denominator = 2.0 * (i as f32 - j as f32);

    // Safety check for near-zero denominator
    if denominator.abs() < f32::EPSILON {
        return f32::INFINITY;
    }

    numerator / denominator
}

/// Computes the 2D Euclidean Distance Transform for an unsigned (binary) mask.
fn edt_unsigned(mask: &[u8], w: usize, h: usize) -> Vec<f32> {
    debug_assert_eq!(mask.len(), w * h, "Mask size must match dimensions");

    // Initialize with a large value for foreground, 0 for background
    // Use maximum possible distance in the image (diagonal from corner to corner)
    let max_dist_squared = (w * w + h * h) as f32;
    let mut f = vec![max_dist_squared; w * h];

    for (i, &m) in mask.iter().enumerate() {
        if m == 0 {
            f[i] = 0.0;
        }
    }

    // First pass: process rows
    let mut row_buffer = vec![0.0; w];
    for y in 0..h {
        let row_start = y * w;
        let row_end = row_start + w;

        // Process this row
        let row_slice = &f[row_start..row_end];
        edt_1d(row_slice, &mut row_buffer);

        // Copy result back
        f[row_start..row_end].copy_from_slice(&row_buffer);
    }

    // Second pass: process columns
    let mut col_input = vec![0.0; h];
    let mut col_output = vec![0.0; h];

    for x in 0..w {
        // Extract column
        for y in 0..h {
            col_input[y] = f[y * w + x];
        }

        // Transform column
        edt_1d(&col_input, &mut col_output);

        // Write back
        for y in 0..h {
            f[y * w + x] = col_output[y];
        }
    }

    // Convert squared distances to actual distances
    for val in &mut f {
        *val = val.sqrt();
    }

    f
}

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use super::*;
    use crate::fieldgraph::compiler::{CompileOptions, FieldGraphCompiler};
    use crate::fieldgraph::spec::FieldGraphSpec;
    use crate::fieldgraph::texture::{Texture, TextureChannel, TextureRegistry};

    #[test]
    fn edt_1d_computes_squared_distance_to_nearest_zero() {
        // Use a large finite value instead of infinity
        let large_val = 1000.0;
        let f = vec![0.0, large_val, large_val, 0.0];
        let mut output = vec![0.0; 4];
        super::edt_1d(&f, &mut output);
        assert_eq!(output, vec![0.0, 1.0, 1.0, 0.0]);
    }

    #[test]
    fn edt_unsigned_returns_rooted_distances() {
        let mask = [0, 1, 1];
        let result = edt_unsigned(&mask, 3, 1);
        let expected = vec![0.0, 1.0, 2.0];
        assert_eq!(result, expected);
    }

    #[test]
    fn edt_handles_all_foreground() {
        let mask = vec![1, 1, 1, 1];
        let result = edt_unsigned(&mask, 2, 2);
        // All pixels should have maximum distance (diagonal of 2x2 grid)
        let expected_distance = (2.0_f32 * 2.0 + 2.0 * 2.0).sqrt();
        for &val in &result {
            assert!((val - expected_distance).abs() < 0.01);
        }
    }

    #[test]
    fn edt_handles_all_background() {
        let mask = vec![0, 0, 0, 0];
        let result = edt_unsigned(&mask, 2, 2);
        // All pixels should have distance 0
        assert_eq!(result, vec![0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn edt_handles_single_pixel() {
        let mask = [1];
        let result = edt_unsigned(&mask, 1, 1);
        // Single foreground pixel has no background neighbor
        assert_eq!(result.len(), 1);
        assert!(result[0] > 0.0);
    }

    #[test]
    fn intersection_handles_same_indices() {
        let f = vec![0.0, 1.0, 4.0];
        let result = intersection_safe(1, 1, &f);
        assert_eq!(result, f32::INFINITY);
    }

    #[test]
    fn intersection_handles_invalid_values() {
        let f = vec![f32::NAN, 1.0];
        let result = intersection_safe(0, 1, &f);
        assert_eq!(result, f32::INFINITY);
    }

    struct MaskTexture;

    impl Texture for MaskTexture {
        fn sample(&self, _channel: TextureChannel, p: Vec2) -> f32 {
            if p.x >= 0.0 {
                1.0
            } else {
                0.0
            }
        }
    }

    #[test]
    fn bake_edt_normalize_generates_normalized_raster() {
        let mut spec = FieldGraphSpec::default();
        spec.add(
            "mask",
            crate::fieldgraph::NodeSpec::texture("mask_tex", TextureChannel::R),
        );

        let program = FieldGraphCompiler::compile(&spec, &CompileOptions::default()).unwrap();
        let mut textures = TextureRegistry::new();
        textures.register("mask_tex", MaskTexture);

        let mut runtime = FieldRuntime::new(std::sync::Arc::new(program), &textures);
        let grid = ChunkGrid {
            origin_domain: Vec2::new(-1.0, 0.0),
            cell_size: 1.0,
            width: 2,
            height: 1,
            halo: 0,
        };

        let raster =
            bake_edt_normalize_params(&mut runtime, "mask", 0.5, 1.0, ChunkId(0, 0), &grid);

        assert_eq!(raster.size(), (2, 1));
        assert_eq!(raster.data, vec![0.0, 1.0]);
    }

    #[test]
    fn edt_produces_correct_distances_for_simple_pattern() {
        // Create a 5x5 mask with a single background pixel in the center
        let mut mask = vec![1; 25];
        mask[12] = 0; // Center pixel (2,2) in a 5x5 grid

        let result = super::edt_unsigned(&mask, 5, 5);

        // Check that center has distance 0
        assert_eq!(result[12], 0.0);

        // Check that adjacent pixels have distance 1
        assert!((result[7] - 1.0).abs() < 0.01); // Above
        assert!((result[17] - 1.0).abs() < 0.01); // Below
        assert!((result[11] - 1.0).abs() < 0.01); // Left
        assert!((result[13] - 1.0).abs() < 0.01); // Right

        // Check that diagonal pixels have distance sqrt(2)
        let sqrt2 = 2.0_f32.sqrt();
        assert!((result[6] - sqrt2).abs() < 0.01); // Top-left
        assert!((result[8] - sqrt2).abs() < 0.01); // Top-right
        assert!((result[16] - sqrt2).abs() < 0.01); // Bottom-left
        assert!((result[18] - sqrt2).abs() < 0.01); // Bottom-right
    }
}
