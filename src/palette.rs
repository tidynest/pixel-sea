//! Ordered dithering, nearest-colour matching, and palette derivation.
//!
//! Quantisation is deliberately the LAST step of the pipeline: the sea is
//! simulated and lit in linear light, tone-mapped, and only then crushed onto
//! a palette. Simulation and visual style stay independent, so a new palette
//! never requires touching a wave.

/// 4x4 ordered (Bayer) threshold matrix, values 0..15.
const BAYER4: [u8; 16] = [
    0, 8, 2, 10, //
    12, 4, 14, 6, //
    3, 11, 1, 9, //
    15, 7, 13, 5,
];

/// Per-pixel colour offset applied before matching. A fixed screen-space
/// pattern, not noise: it is identical in every frame, so it reads as texture.
/// Random noise per frame would crawl and boil.
#[inline]
pub fn dither_bias(x: u32, y: u32, strength: f32) -> f32 {
    let cell = BAYER4[((y & 3) * 4 + (x & 3)) as usize] as f32;
    (cell / 16.0 - 0.468_75) * strength
}

/// Index of the closest palette entry, weighted towards green because the eye
/// is. Returning an index rather than a colour is what lets the encoder write
/// an indexed PNG: one byte per pixel instead of three, and dithered indices
/// compress far better than dithered RGB triples.
#[inline]
pub fn nearest_index(palette: &[u32], r: f32, g: f32, b: f32) -> u8 {
    let (r, g, b) = (
        r.clamp(0.0, 255.0),
        g.clamp(0.0, 255.0),
        b.clamp(0.0, 255.0),
    );
    let mut best = 0usize;
    let mut best_d = f32::MAX;
    for (i, &c) in palette.iter().enumerate() {
        let dr = ((c >> 16) & 0xff) as f32 - r;
        let dg = ((c >> 8) & 0xff) as f32 - g;
        let db = (c & 0xff) as f32 - b;
        let d = 2.0 * dr * dr + 4.0 * dg * dg + 3.0 * db * db;
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    best as u8
}

/// Flatten a palette into the RGB triples a PNG PLTE chunk expects.
pub fn plte(palette: &[u32]) -> Vec<u8> {
    palette
        .iter()
        .flat_map(|&c| [(c >> 16) as u8, (c >> 8) as u8, c as u8])
        .collect()
}

/// Derive a palette of `n` colours from actual rendered pixels, by median cut.
///
/// Hand-picking hex values that happen to match what the renderer produces is
/// a losing game - change the sun's elevation and every one of them is subtly
/// wrong. Sampling the render itself cannot drift out of sync, and it makes
/// the palette size a plain dial rather than a rewrite.
///
/// Repeatedly split the box with the widest colour spread, at the median of
/// its widest axis, then average each box. Deterministic for a given input.
pub fn median_cut(px: &mut [[u8; 3]], n: usize) -> Vec<u32> {
    assert!((2..=256).contains(&n), "palette must be 2..=256 colours");
    assert!(!px.is_empty(), "no pixels to derive a palette from");

    let mut boxes = vec![(0usize, px.len())];
    while boxes.len() < n {
        let Some((bi, axis)) = boxes
            .iter()
            .enumerate()
            .filter(|&(_, &(a, b))| b - a >= 2)
            .map(|(i, &(a, b))| {
                let (axis, spread) = widest_axis(&px[a..b]);
                (i, axis, spread)
            })
            .max_by(|x, y| x.2.total_cmp(&y.2))
            .map(|(i, axis, _)| (i, axis))
        else {
            break; // every box is a single colour; nothing left to split
        };

        let (a, b) = boxes[bi];
        px[a..b].sort_unstable_by_key(|c| c[axis]);
        let mid = a + (b - a) / 2;
        boxes[bi] = (a, mid);
        boxes.push((mid, b));
    }

    boxes.iter().map(|&(a, b)| average(&px[a..b])).collect()
}

fn widest_axis(px: &[[u8; 3]]) -> (usize, f32) {
    let (mut lo, mut hi) = ([255u8; 3], [0u8; 3]);
    for c in px {
        for k in 0..3 {
            lo[k] = lo[k].min(c[k]);
            hi[k] = hi[k].max(c[k]);
        }
    }
    // Weighted to match `nearest_index`, so cuts land where the eye looks.
    let w = [2.0f32, 4.0, 3.0];
    let mut best = (0usize, -1.0f32);
    for k in 0..3 {
        let s = (hi[k] - lo[k]) as f32 * w[k];
        if s > best.1 {
            best = (k, s);
        }
    }
    best
}

fn average(px: &[[u8; 3]]) -> u32 {
    let n = px.len().max(1) as u32;
    let sum = px.iter().fold([0u32; 3], |mut a, c| {
        for k in 0..3 {
            a[k] += c[k] as u32;
        }
        a
    });
    (sum[0] / n) << 16 | (sum[1] / n) << 8 | (sum[2] / n)
}
