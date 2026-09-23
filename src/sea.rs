//! The sea: wave field, perspective, sky, sun, and water shading.
//!
//! All shading happens in LINEAR light and is tone-mapped once at the end.
//! Mixing colours in display space is what makes CG gradients go muddy and
//! highlights go grey, and no amount of palette work recovers from it.
//!
//! Nothing in this file knows about files, PNGs, or the command line.

use crate::palette;
use crate::theme::{Theme, lin};
use std::f32::consts::TAU;

// ---------------------------------------------------------------- style ----

/// How the finished colour is committed to bytes.
pub enum Quant {
    /// Full 24-bit. No banding, no dither.
    None,
    /// Snap to a palette. `dither` is in colour units; 0 gives hard bands.
    Palette { colors: Vec<u32>, dither: f32 },
}

pub struct Style {
    pub w: u32,
    pub h: u32,
    pub quant: Quant,
}

impl Style {
    /// One byte per pixel for palette styles (an index), three for RGB.
    pub fn bytes_per_pixel(&self) -> usize {
        match self.quant {
            Quant::None => 3,
            Quant::Palette { .. } => 1,
        }
    }

    pub fn buffer_len(&self) -> usize {
        self.w as usize * self.h as usize * self.bytes_per_pixel()
    }
}

// ---------------------------------------------------------------- waves ----

/// A wave with its temporal frequency snapped to a harmonic of the loop.
pub struct SnapWave {
    amp: f32,
    kx: f32,
    kz: f32,
    omega: f32,
    phase: f32,
    wavelength: f32,
}

const GRAVITY: f32 = 9.81;

/// How often the sparkle pattern re-rolls, Hz. Must divide the loop evenly.
const SPARKLE_HZ: f32 = 6.0;

// ----------------------------------------------------------------- prep ----

/// Deep-water dispersion: `c = sqrt(g * lambda / 2pi)`. Long swells outrun
/// short chop by a wide margin, and that spread of speeds is most of why real
/// water reads as water while a bank of same-speed sines reads as corduroy.
fn phase_speed(wavelength: f32) -> f32 {
    (GRAVITY * wavelength / TAU).sqrt()
}

/// Build the wave field for a theme, every component forced onto an exact
/// harmonic of the loop.
///
/// A wave returns to its start after `2*PI / omega`. If every wave's period
/// divides `loop_secs`, the whole surface returns to its starting state at
/// `loop_secs` - so the last frame joins the first with no seam. The nudge is
/// at most half a harmonic step and is not visible.
///
/// Directions fan by golden-ratio offsets rather than evenly, because an even
/// fan still lets components line up periodically and stripe.
pub fn snap(theme: &Theme, loop_secs: f32) -> Vec<SnapWave> {
    let mut out = Vec::new();
    for sys in [&theme.swell, &theme.wind] {
        for (i, &(amp, wavelength)) in sys.octaves.iter().enumerate() {
            let g = (i as f32 * 0.618_034).fract() * 2.0 - 1.0;
            let dir = (sys.dir_deg + sys.spread_deg * g).to_radians();
            let k = TAU / wavelength;
            let wanted = k * phase_speed(wavelength) * theme.time_scale;
            let harmonics = (wanted * loop_secs / TAU).round().max(1.0);
            out.push(SnapWave {
                amp,
                kx: k * dir.sin(),
                kz: k * dir.cos(),
                omega: harmonics * TAU / loop_secs,
                phase: out.len() as f32 * 2.399_963,
                wavelength,
            });
        }
    }
    out
}

/// Trochoidal-ish wave profile and its derivative with respect to phase.
///
/// A plain sine is symmetric. Real gravity waves have peaked crests over long
/// flat troughs, and that asymmetry is one of the strongest "this is water"
/// cues there is. Squaring the half-shifted sine does it in three multiplies
/// and keeps the derivative closed-form, which is what the lighting needs.
#[inline]
fn shape(sin_p: f32, cos_p: f32) -> (f32, f32) {
    let s = 0.5 * (sin_p + 1.0);
    (2.0 * s * s - 1.0, 2.0 * s * cos_p)
}

// -------------------------------------------------------------- lighting ----

/// Atmospheric extinction along a ray at the given elevation.
///
/// A sun three degrees up is seen through many times more air than one
/// overhead, and blue is scattered out of the beam first. This is why a
/// sunset is red and dim - and deriving it rather than picking an orange by
/// hand is what keeps the disc, its aureole and its reflection in agreement.
fn extinction(sin_elev: f32, k: [f32; 3]) -> [f32; 3] {
    let airmass = 1.0 / (sin_elev.max(0.01) + 0.12);
    [
        (-k[0] * airmass).exp(),
        (-k[1] * airmass).exp(),
        (-k[2] * airmass).exp(),
    ]
}

/// The vertical colour ramp of the sky, given the sine of an elevation angle.
///
/// Three stops rather than two: warm at the horizon, violet through the
/// middle, saturated indigo overhead. The violet stop is what stops a sunset
/// sky reading as a single orange-to-black smear.
fn sky_gradient(s: &Scene, sin_elev: f32) -> [f32; 3] {
    let e = sin_elev.max(-0.05);
    let t = smoothstep(0.0, 0.75, e).powf(0.65);
    let base = if t < 0.45 {
        lerp3(s.horizon, s.mid, t / 0.45)
    } else {
        lerp3(s.mid, s.zenith, (t - 0.45) / 0.55)
    };
    // Warm glow piled into the first few degrees, where the line of sight
    // runs longest through dusty air.
    let haze = (1.0 - smoothstep(0.0, 0.16, e)).powi(2);
    add3(base, mul3(s.haze, haze * s.th.sky.haze_k))
}

/// Radiance arriving from the sun along a ray, given the cosine of the angle
/// between that ray and the sun's bearing.
///
/// Used for the sun in the sky AND for its reflection in the water. That is
/// why the glitter path needs no code of its own: it is this one function
/// evaluated across a few hundred thousand differently-tilted facets, and the
/// tapering road falls out because nearer water tilts more.
fn sun_radiance(cd: f32, s: &Scene) -> [f32; 3] {
    let r = s.th.light.ang_r;
    let (ts, tw, ws, ww) = s.th.light.aureole;
    let ang = (2.0 * (1.0 - cd).max(0.0)).sqrt(); // chord ~ angle, small angles
    let disc = 1.0 - smoothstep(r * 0.8, r * 1.2, ang);
    mul3(
        s.sun_color,
        disc + (-ang / ts).exp() * tw + (-ang / ws).exp() * ww,
    )
}

/// Cumulus deck coverage along a ray, 0..1.
///
/// The clouds sit on a flat deck at one altitude, so looking further down
/// toward the horizon is looking further away ALONG that deck. Dividing by
/// the elevation is that perspective, and it is what makes the shapes crowd
/// and flatten toward the horizon instead of tiling evenly across the sky.
///
/// Static on purpose: over a 20 s loop real cloud drift is imperceptible, it
/// costs nothing, and it cannot break the seam.
fn cloud_cover(d: [f32; 3], s: &Scene) -> f32 {
    let e = d[1];
    if e < 0.008 {
        return 0.0;
    }
    let c = &s.th.clouds;
    let along = 1.0 / (e + 0.035);
    let across = d[0] / d[2].max(0.05) * along;
    smoothstep(c.lo, c.hi, fbm(across * c.scale, along * c.scale))
        * smoothstep(0.008, 0.045, e)
        * (1.0 - smoothstep(0.35, 0.80, e))
}

/// Fixed star field.
///
/// Hashed on a direction quantised to roughly one screen pixel, so stars hold
/// still between frames instead of crawling, and so they stay one pixel at any
/// render size. Faded into the horizon haze and hidden behind cloud, like real
/// ones.
fn stars(d: [f32; 3], s: &Scene) -> f32 {
    let density = s.th.sky.stars;
    if density <= 0.0 || d[1] < 0.02 {
        return 0.0;
    }
    let h = hash2(
        (d[0] * s.focal) as i32 as u32,
        (d[1] * s.focal) as i32 as u32,
    );
    if h < 1.0 - density {
        return 0.0;
    }
    ((h - (1.0 - density)) / density).powf(0.45) * smoothstep(0.02, 0.16, d[1])
}

/// Everything the sky sends along a ray: gradient, sun, and cloud deck.
///
/// This is THE function the water reflects. Feeding the reflected ray through
/// the same code the sky itself uses is what puts the clouds into the water -
/// a sky gradient alone reflects as a flat wash, and a flat wash is why CG
/// water so often reads as tinted glass instead of a mirror.
///
/// `sun_scale` lets the water dim or brighten the solar term per pixel, which
/// is where the sparkle grain comes from.
fn sky_radiance(d: [f32; 3], s: &Scene, sun_scale: f32) -> [f32; 3] {
    let mut c = add3(
        sky_gradient(s, d[1]),
        mul3(sun_radiance(dot(d, s.sun_dir), s), sun_scale),
    );
    c = add3(c, mul3([0.62, 0.72, 1.0], stars(d, s) * 2.4));
    let cover = cloud_cover(d, s);
    if cover > 0.0 {
        // Cumulus at this hour are in shadow except where they face the sun
        // and where the low light rakes along their bases. That split is what
        // makes them read as volumes rather than grey cut-outs.
        let facing = smoothstep(0.80, 0.999, dot(d, s.sun_dir));
        let under = 1.0 - smoothstep(0.02, 0.26, d[1]);
        let body = lerp3(s.cloud_dark, s.cloud_lit, (facing + under * 0.55).min(1.0));
        c = lerp3(c, body, cover * s.th.clouds.opacity);
    }
    c
}

// --------------------------------------------------------------- render ----

/// Per-frame constants shared by every pixel.
struct Scene {
    th: &'static Theme,
    t: f32,
    hy: f32,
    focal: f32,
    half_w: f32,
    crest: f32,
    tick: u32,
    sun_dir: [f32; 3],
    sun_color: [f32; 3],
    sky_amb: [f32; 3],
    // Theme colours, converted from sRGB hex to linear once per frame rather
    // than once per pixel.
    zenith: [f32; 3],
    mid: [f32; 3],
    horizon: [f32; 3],
    haze: [f32; 3],
    cloud_dark: [f32; 3],
    cloud_lit: [f32; 3],
    water_deep: [f32; 3],
    water_lit: [f32; 3],
    sss: [f32; 3],
    foam: [f32; 3],
}

/// Render one frame into `buf`: one byte per pixel for palette styles (a
/// palette index), three for `Quant::None` (RGB).
pub fn render(
    buf: &mut [u8],
    style: &Style,
    th: &'static Theme,
    waves: &[SnapWave],
    t: f32,
    loop_secs: f32,
) {
    let (w, h) = (style.w, style.h);
    assert_eq!(buf.len(), style.buffer_len(), "buffer does not match style");

    let hy = h as f32 * th.horizon_frac;
    let focal = h as f32;
    let steps = (SPARKLE_HZ * loop_secs).round().max(1.0);
    let elev = th.light.elev;
    let mut s = Scene {
        th,
        t,
        hy,
        focal,
        half_w: w as f32 * 0.5,
        crest: waves.iter().map(|v| v.amp).sum::<f32>().max(1e-4),
        // Sparkle ticks a whole number of times per loop, so it repeats too.
        tick: ((t / loop_secs * steps).floor() as i64).rem_euclid(steps as i64) as u32,
        sun_dir: [0.0, elev.sin(), elev.cos()],
        sun_color: mul3(
            mul3v(extinction(elev.sin(), th.light.airmass_k), th.light.tint),
            th.light.intensity,
        ),
        sky_amb: [0.0; 3], // filled in below, once the ramp colours exist
        zenith: lin(th.sky.zenith),
        mid: lin(th.sky.mid),
        horizon: lin(th.sky.horizon),
        haze: lin(th.sky.haze),
        cloud_dark: lin(th.clouds.dark),
        cloud_lit: lin(th.clouds.lit),
        water_deep: lin(th.water.deep),
        water_lit: lin(th.water.lit),
        sss: lin(th.water.sss),
        foam: lin(th.water.foam),
    };
    s.sky_amb = sky_gradient(&s, 0.45);

    let bpp = style.bytes_per_pixel();
    for py in 0..h {
        let sy = (py as f32 + 0.5) - hy;
        // Distance to the water at this row, and how many metres of sea one
        // pixel row covers there. The second number is what stops the horizon
        // turning into a buzzing moire mess.
        let (z, footprint) = if sy > 0.5 {
            let z = th.eye_h * focal / sy;
            (z, z * z / (th.eye_h * focal))
        } else {
            (0.0, 0.0)
        };

        for px in 0..w {
            let lit = if sy > 0.5 {
                sea_pixel(px, py, z, footprint, waves, &s)
            } else {
                sky_pixel(px, py, &s)
            };
            let c = to_display(lit);

            let i = ((py * w + px) as usize) * bpp;
            match &style.quant {
                Quant::None => {
                    buf[i] = c[0] as u8;
                    buf[i + 1] = c[1] as u8;
                    buf[i + 2] = c[2] as u8;
                }
                Quant::Palette { colors, dither } => {
                    let b = palette::dither_bias(px, py, *dither);
                    buf[i] = palette::nearest_index(colors, c[0] + b, c[1] + b, c[2] + b);
                }
            }
        }
    }
}

fn sky_pixel(px: u32, py: u32, s: &Scene) -> [f32; 3] {
    let xa = (px as f32 + 0.5 - s.half_w) / s.focal;
    let ya = (s.hy - (py as f32 + 0.5)) / s.focal;
    let inv = 1.0 / (xa * xa + ya * ya + 1.0).sqrt();
    sky_radiance([xa * inv, ya * inv, inv], s, 1.0)
}

fn sea_pixel(px: u32, py: u32, z: f32, footprint: f32, waves: &[SnapWave], s: &Scene) -> [f32; 3] {
    let x = (px as f32 + 0.5 - s.half_w) * z / s.focal;

    // Sum the wave field and its exact slope in the same pass. The derivative
    // of the profile is closed-form, so the surface normal costs a couple of
    // multiplies per wave instead of a second round of sampling.
    let (mut height, mut dhdx, mut dhdz) = (0.0f32, 0.0f32, 0.0f32);
    for v in waves {
        // Drop any wave finer than the pixels can show, or it aliases.
        let att = smoothstep(1.0, 3.0, v.wavelength / footprint.max(1e-4));
        if att <= 0.0 {
            continue;
        }
        let p = v.kx * x + v.kz * z + v.omega * s.t + v.phase;
        let (sin_p, cos_p) = p.sin_cos();
        let (prof, dprof) = shape(sin_p, cos_p);
        let a = v.amp * att;
        height += a * prof;
        dhdx += a * v.kx * dprof;
        dhdz += a * v.kz * dprof;
    }

    let n = normalize([-dhdx, 1.0, -dhdz]);
    let view = normalize([-x, s.th.eye_h - height, -z]);
    let ndv = dot(n, view).max(1e-4);

    // Mirror the view about the surface normal and ask the sky what lies in
    // that direction. This one lookup replaces every hand-drawn reflection:
    // the glitter road, the warm column, the horizon mirror are all just this
    // evaluated across facets that tilt more the nearer they are.
    let r = [
        2.0 * ndv * n[0] - view[0],
        2.0 * ndv * n[1] - view[1],
        2.0 * ndv * n[2] - view[2],
    ];
    let sparkle = 0.35 + 0.65 * smoothstep(0.25, 0.80, hash3(px, py, s.tick));
    let reflected = sky_radiance(normalize(r), s, sparkle);

    // Schlick, with no fudge factor. With the eye 14 m up, near water really
    // does reflect only a few percent, so the dark body shows through, while
    // the far field goes to a near-perfect mirror. That contrast is the shot.
    let fres = 0.02 + 0.98 * (1.0 - ndv).powi(5);

    let lift = ((height / s.crest) * 0.5 + 0.5).clamp(0.0, 1.0);
    let body = add3(
        lerp3(s.water_deep, s.water_lit, lift),
        mul3(s.sky_amb, 0.05),
    );

    // Light that entered a crest and came out the front. Strongest on thin,
    // sun-facing faces. This is the glow inside a backlit swell, and it is
    // most of what separates water from coloured vinyl.
    let sss = lift.powi(3) * smoothstep(0.0, 0.30, dhdz);
    let mut c = add3(
        lerp3(body, reflected, fres),
        mul3(s.sss, sss * s.sun_color[0] * s.th.water.sss_k),
    );

    // Whitecaps break where the surface is STEEP, not where it is high.
    let slope = (dhdx * dhdx + dhdz * dhdz).sqrt();
    let cap = smoothstep(0.55, 1.10, slope)
        * smoothstep(0.40, 0.80, hash3(px ^ 0x9e37, py, s.tick))
        * s.th.water.foam_k;
    c = lerp3(c, mul3(s.foam, 0.22), cap.min(1.0));

    // Aerial perspective: distant water dissolves into the horizon haze.
    lerp3(
        c,
        sky_gradient(s, 0.002),
        smoothstep(800.0, 12000.0, z) * 0.75,
    )
}

/// Linear radiance to display bytes.
///
/// Reinhard rolloff, then gamma 2.0. Without the rolloff the sun clips to a
/// flat white plate and drags the whole top end of the sky up with it; the
/// rolloff is what lets the disc be genuinely brighter than the sky and still
/// fit in a byte.
#[inline]
fn to_display(c: [f32; 3]) -> [f32; 3] {
    let f = |v: f32| {
        let v = v.max(0.0);
        (v / (1.0 + v)).sqrt() * 255.0
    };
    [f(c[0]), f(c[1]), f(c[2])]
}

// ---------------------------------------------------------------- maths ----

#[inline]
fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

#[inline]
fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

#[inline]
fn mul3(a: [f32; 3], k: f32) -> [f32; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}

#[inline]
fn mul3v(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2]]
}

#[inline]
fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Deterministic per-pixel-tick value in 0..1. Drives the sparkle.
#[inline]
fn hash3(x: u32, y: u32, f: u32) -> f32 {
    let mut h =
        x.wrapping_mul(0x27d4_eb2d) ^ y.wrapping_mul(0x1656_67b1) ^ f.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    (h >> 8) as f32 / 16_777_216.0
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
fn hash2(x: u32, y: u32) -> f32 {
    let mut h = x.wrapping_mul(0x27d4_eb2d) ^ y.wrapping_mul(0x8548_1ad7);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 13;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 16;
    (h >> 8) as f32 / 16_777_216.0
}

/// Bilinear value noise with a smoothstep fade, so the lattice does not show.
fn vnoise(x: f32, y: f32) -> f32 {
    let (xi, yi) = (x.floor(), y.floor());
    let (fx, fy) = (x - xi, y - yi);
    let u = fx * fx * (3.0 - 2.0 * fx);
    let v = fy * fy * (3.0 - 2.0 * fy);
    let (ix, iy) = (xi as i32 as u32, yi as i32 as u32);
    lerp(
        lerp(hash2(ix, iy), hash2(ix.wrapping_add(1), iy), u),
        lerp(
            hash2(ix, iy.wrapping_add(1)),
            hash2(ix.wrapping_add(1), iy.wrapping_add(1)),
            u,
        ),
        v,
    )
}

/// Four octaves of value noise.
///
/// Cumulus are self-similar - big cauliflower lobes with smaller lobes stuck
/// to them - which is precisely what summing halving amplitudes at doubling
/// frequencies produces. One octave gives blobs; four gives clouds.
fn fbm(x: f32, y: f32) -> f32 {
    let (mut sum, mut amp, mut px, mut py) = (0.0, 0.5, x, y);
    for _ in 0..4 {
        sum += amp * vnoise(px, py);
        px *= 2.03;
        py *= 2.03;
        amp *= 0.5;
    }
    sum / 0.9375
}

#[inline]
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn normalize(v: [f32; 3]) -> [f32; 3] {
    let m = dot(v, v).sqrt().max(1e-6);
    [v[0] / m, v[1] / m, v[2] / m]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::THEMES;

    const T: f32 = 20.0;
    const W: u32 = 96;
    const H: u32 = 54;

    fn th() -> &'static Theme {
        &THEMES[0]
    }

    fn frame(quant: Quant, t: f32) -> Vec<u8> {
        let style = Style { w: W, h: H, quant };
        let mut buf = vec![0u8; style.buffer_len()];
        render(&mut buf, &style, th(), &snap(th(), T), t, T);
        buf
    }

    /// The load-bearing one. If a wave is added and `snap` is bypassed, the
    /// loop gets a visible jolt once every T seconds - this fails instead.
    /// Checked across every theme, since each carries its own spectrum.
    #[test]
    fn every_frequency_is_a_loop_harmonic() {
        for theme in THEMES {
            for v in snap(theme, T) {
                let n = v.omega * T / TAU;
                assert!(n.round() >= 1.0, "{}: wave is frozen, n = {n}", theme.name);
                assert!(
                    (n - n.round()).abs() < 1e-4,
                    "{}: omega {} is not harmonic",
                    theme.name,
                    v.omega
                );
            }
        }
    }

    #[test]
    fn animation_repeats_after_one_loop() {
        let a = frame(Quant::None, 3.1);
        let b = frame(Quant::None, 3.1 + T);
        let worst = a.iter().zip(&b).map(|(x, y)| x.abs_diff(*y)).max().unwrap();
        assert!(worst <= 2, "seam in the loop: worst channel drift {worst}");
    }

    #[test]
    fn palette_output_stays_on_palette() {
        let rgb = frame(Quant::None, 2.0);
        let mut px: Vec<[u8; 3]> = rgb.as_chunks::<3>().0.to_vec();
        let colors = palette::median_cut(&mut px, 48);
        assert_eq!(colors.len(), 48, "median cut returned the wrong count");

        let buf = frame(
            Quant::Palette {
                colors,
                dither: 24.0,
            },
            2.0,
        );
        for &i in &buf {
            assert!((i as usize) < 48, "index {i} points off the end");
        }
        let used = buf.iter().collect::<std::collections::HashSet<_>>().len();
        assert!(used >= 12, "only {used} entries used - render is flat");
    }

    /// Every theme must produce a scene with real range in it. This is the net
    /// that catches a new theme whose colours cancel into mud, or whose light
    /// is so dim the frame comes out black.
    #[test]
    fn every_theme_renders_something() {
        for theme in THEMES {
            let style = Style {
                w: W,
                h: H,
                quant: Quant::None,
            };
            let mut buf = vec![0u8; style.buffer_len()];
            render(&mut buf, &style, theme, &snap(theme, T), T * 0.37, T);
            let (lo, hi) = (*buf.iter().min().unwrap(), *buf.iter().max().unwrap());
            assert!(hi - lo > 60, "{} is flat: range {lo}..{hi}", theme.name);
            assert!(hi > 90, "{} is too dark to see: max {hi}", theme.name);
        }
    }

    #[test]
    fn theme_names_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for theme in THEMES {
            assert!(
                seen.insert(theme.name),
                "duplicate theme name {}",
                theme.name
            );
        }
    }
}
