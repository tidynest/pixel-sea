mod palette;
mod sea;
mod theme;

use sea::{Quant, SnapWave, Style};
use std::fs::File;
use std::io::{BufWriter, Write};
use theme::Theme;

fn usage() -> String {
    format!(
        "\
pixel-sea - bakes a seamless pixel-art sea loop to an animated PNG

    pixel-sea [--theme T] [--style S] [--colors N] [--seconds N] [--fps N]
              [--still] [--at F] [--out FILE]

    --theme    {}
    --style    retro    320x180, 64 colours   (default)
               retro16  320x180, 16 colours
               chunky   320x180, 24-bit, no dither
               fine     640x360, 96 colours
    --width    override the style's pixel width, 16..=4096
    --height   override the style's pixel height, 16..=4096. Aspect is free:
               the vertical field of view is fixed, so a square crops the
               scene in from the sides and a wide strip opens it out.
    --colors   override the palette size, 2..=256. The palette is derived
               from the render itself, so this is a plain dial - there are no
               hand-written hex tables to keep in sync.
    --seconds  loop length, default 20. Longer is not just less repetitive:
               it gives the harmonic snapper a finer grid, so the long swells
               stop collapsing onto one shared frequency and beating together.
    --fps      frames per second, default 12
    --list     print the themes with a one-line description of each
    --still    write a single frame instead of a loop. Wave frequencies are
               still snapped against --seconds, so the still matches what the
               animation looks like. Use it to judge a theme in a second.
    --at       which moment of the loop the still comes from, 0.0..=1.0 as a
               fraction of --seconds. Default 0.37. Implies --still, since it
               means nothing for an animation. Scrub it to pick the frame you
               want: the glitter and the crest lighting move a lot.
    --out      output path, default sea_<theme>_<style>.png
",
        theme::names()
    )
}

/// Derive the palette from what the renderer actually produces.
///
/// Sampled across three moments of the loop, because the sparkle and the
/// crest lighting move: a palette cut from a single frame starves whichever
/// highlights that frame happened not to contain.
fn build_palette(
    w: u32,
    h: u32,
    th: &'static Theme,
    waves: &[SnapWave],
    secs: f32,
    n: usize,
) -> Vec<u32> {
    let probe = Style {
        w,
        h,
        quant: Quant::None,
    };
    let mut buf = vec![0u8; probe.buffer_len()];
    let mut px: Vec<[u8; 3]> = Vec::with_capacity(3 * (w * h) as usize);
    for k in 0..3 {
        let t = secs * (0.11 + k as f32 / 3.0);
        sea::render(&mut buf, &probe, th, waves, t, secs);
        px.extend(buf.as_chunks::<3>().0.iter().copied());
    }
    palette::median_cut(&mut px, n)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut theme_name = "dusk".to_string();
    let mut style_name = "retro".to_string();
    let (mut secs, mut fps) = (20.0f32, 12u16);
    let (mut out, mut colors, mut still) = (None::<String>, None::<usize>, false);
    let mut at = 0.37f32;
    let (mut width, mut height) = (None::<u32>, None::<u32>);

    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut val = || args.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--theme" => theme_name = val()?,
            "--style" => style_name = val()?,
            "--colors" => colors = Some(val()?.parse()?),
            "--width" => width = Some(val()?.parse()?),
            "--height" => height = Some(val()?.parse()?),
            "--seconds" => secs = val()?.parse()?,
            "--fps" => fps = val()?.parse()?,
            "--out" => out = Some(val()?),
            "--still" => still = true,
            "--at" => {
                at = val()?.parse()?;
                still = true;
            }
            "--list" => {
                for th in theme::THEMES {
                    println!("{:<9} {}", th.name, th.blurb);
                }
                return Ok(());
            }
            "-h" | "--help" => {
                print!("{}", usage());
                return Ok(());
            }
            other => return Err(format!("unknown argument: {other}\n\n{}", usage()).into()),
        }
    }

    let th = theme::by_name(&theme_name)
        .ok_or_else(|| format!("unknown theme: {theme_name} ({})", theme::names()))?;

    let (def_w, def_h, default_colors) = match style_name.as_str() {
        "retro" => (320, 180, Some(64)),
        "retro16" => (320, 180, Some(16)),
        "chunky" => (320, 180, None),
        "fine" => (640, 360, Some(96)),
        o => return Err(format!("unknown style: {o} (retro | retro16 | chunky | fine)").into()),
    };

    let (w, h) = (width.unwrap_or(def_w), height.unwrap_or(def_h));
    if !(16..=4096).contains(&w) || !(16..=4096).contains(&h) {
        return Err("--width and --height must be between 16 and 4096".into());
    }

    if !(0.5..=120.0).contains(&secs) {
        return Err("--seconds must be between 0.5 and 120".into());
    }
    if !(1..=60).contains(&fps) {
        return Err("--fps must be between 1 and 60".into());
    }
    if !(0.0..=1.0).contains(&at) {
        return Err("--at must be between 0.0 and 1.0".into());
    }
    let frames = if still {
        1
    } else {
        (secs * fps as f32).round() as u32
    };
    if frames == 0 {
        return Err("--seconds x --fps rounds to zero frames".into());
    }
    let n_colors = colors.or(default_colors);
    if let Some(n) = n_colors
        && !(2..=256).contains(&n)
    {
        return Err("--colors must be between 2 and 256".into());
    }

    eprintln!("  {} - {}", th.name, th.blurb);
    let waves = sea::snap(th, secs);
    if waves.is_empty() {
        return Err(format!("theme {theme_name} has no waves - the sea would be flat").into());
    }

    let quant = match n_colors {
        None => Quant::None,
        Some(n) => Quant::Palette {
            colors: build_palette(w, h, th, &waves, secs, n),
            // More colours needs LESS dither, not the same: the whole point of
            // the extra entries is that the gap being hidden is smaller.
            dither: 58.0 / (n as f32).cbrt(),
        },
    };
    let style = Style { w, h, quant };

    let path = out.unwrap_or_else(|| format!("sea_{theme_name}_{style_name}.png"));
    let mut enc = png::Encoder::new(BufWriter::new(File::create(&path)?), style.w, style.h);
    match &style.quant {
        Quant::None => {
            enc.set_color(png::ColorType::Rgb);
            enc.set_adaptive_filter(png::AdaptiveFilterType::Adaptive);
        }
        Quant::Palette { colors, .. } => {
            enc.set_color(png::ColorType::Indexed);
            enc.set_palette(palette::plte(colors));
            // Filtering subtracts neighbouring bytes. On palette *indices*
            // that arithmetic is meaningless and hurts compression badly.
            enc.set_filter(png::FilterType::NoFilter);
        }
    }
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::Best);
    if !still {
        enc.set_animated(frames, 0)?; // 0 plays = loop forever
        enc.set_frame_delay(1, fps)?;
    }
    let mut writer = enc.write_header()?;

    // Known limit: single-threaded. Reach for std::thread::scope over row bands
    // only if the bake starts to annoy you.
    let mut buf = vec![0u8; style.buffer_len()];
    for i in 0..frames {
        let t = if still {
            secs * at
        } else {
            i as f32 * secs / frames as f32
        };
        sea::render(&mut buf, &style, th, &waves, t, secs);
        writer.write_image_data(&buf)?;
        if !still {
            eprint!("\r  {}/{frames}", i + 1);
            std::io::stderr().flush().ok();
        }
    }
    writer.finish()?;
    if still {
        eprintln!("  wrote {path} (still at {at} of {secs}s, {theme_name}/{style_name})");
    } else {
        eprintln!("\r  wrote {path} ({frames} frames, {secs}s loop, {fps} fps)");
    }
    Ok(())
}
