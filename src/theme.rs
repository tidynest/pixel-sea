//! Scene presets: everything that makes one sea a different sea.
//!
//! Colours are written as sRGB hex because that is how anyone choosing them
//! thinks. The renderer needs linear light, so `lin()` converts once per frame
//! rather than making the author do it by hand.
//!
//! Adding a theme means adding a row here. It means touching no rendering code
//! at all, and the palette is cut from the render itself, so a new theme gets
//! a matching palette for free.

/// One wave system: octaves of (amplitude m, wavelength m), coarse to fine,
/// fanned around a mean direction.
pub struct System {
    pub dir_deg: f32,
    pub spread_deg: f32,
    pub octaves: &'static [(f32, f32)],
}

pub struct Sky {
    pub zenith: u32,
    pub mid: u32,
    pub horizon: u32,
    pub haze: u32,
    /// How much warm haze piles into the first few degrees above the horizon.
    pub haze_k: f32,
    /// Fraction of sky pixels carrying a star. 0 for anything daylit.
    pub stars: f32,
}

/// The sun, or the moon - the difference is intensity, size and tint.
pub struct Light {
    /// Elevation above the horizon, radians.
    pub elev: f32,
    /// Angular radius, radians. Everything downstream scales from this.
    pub ang_r: f32,
    /// Peak linear radiance before extinction.
    pub intensity: f32,
    /// Multiplies the extinction result. Cool for a moon, neutral for a sun.
    pub tint: [f32; 3],
    /// Per-channel extinction coefficients. Larger means more of that channel
    /// scattered out of the beam before it reaches the eye, so a big spread
    /// between them reddens the light and a flat set greys it.
    pub airmass_k: [f32; 3],
    /// (tight falloff, tight weight, wide falloff, wide weight).
    pub aureole: (f32, f32, f32, f32),
}

pub struct Clouds {
    pub dark: u32,
    pub lit: u32,
    /// Noise frequency on the cloud deck. Lower means bigger lobes.
    pub scale: f32,
    /// Coverage threshold window. Narrow gives hard cumulus edges, wide gives
    /// soft stratus; low `lo` gives more sky covered.
    pub lo: f32,
    pub hi: f32,
    pub opacity: f32,
}

pub struct Water {
    pub deep: u32,
    pub lit: u32,
    /// Colour of light that passes through a crest and out the front.
    pub sss: u32,
    pub sss_k: f32,
    pub foam: u32,
    /// Whitecap abundance. Storms break constantly; a glassy dawn does not.
    pub foam_k: f32,
}

pub struct Theme {
    pub name: &'static str,
    pub blurb: &'static str,
    /// Eye height, metres. Higher sees further, so waves subtend less and the
    /// sea reads vaster and calmer.
    pub eye_h: f32,
    pub horizon_frac: f32,
    /// Global brake on wave motion. 1.0 is physically correct.
    pub time_scale: f32,
    pub sky: Sky,
    pub light: Light,
    pub clouds: Clouds,
    pub water: Water,
    pub swell: System,
    pub wind: System,
}

/// sRGB hex to linear light, gamma 2.0.
pub fn lin(hex: u32) -> [f32; 3] {
    let f = |shift: u32| {
        let v = ((hex >> shift) & 0xff) as f32 / 255.0;
        v * v
    };
    [f(16), f(8), f(0)]
}

pub fn by_name(name: &str) -> Option<&'static Theme> {
    THEMES.iter().find(|t| t.name == name)
}

pub fn names() -> String {
    THEMES
        .iter()
        .map(|t| t.name)
        .collect::<Vec<_>>()
        .join(" | ")
}

pub const THEMES: &[Theme] = &[
    Theme {
        name: "dusk",
        blurb: "Low sun, violet sky, warm cumulus over deep blue water.",
        eye_h: 14.0,
        horizon_frac: 0.44,
        time_scale: 0.34,
        sky: Sky {
            zenith: 0x1E2878,
            mid: 0x7A4896,
            horizon: 0xE89678,
            haze: 0xFFA05A,
            haze_k: 0.22,
            stars: 0.0,
        },
        light: Light {
            elev: 0.052,
            ang_r: 0.043,
            intensity: 26.0,
            tint: [1.0, 1.0, 1.0],
            airmass_k: [0.13, 0.30, 0.62],
            aureole: (0.050, 0.55, 0.24, 0.07),
        },
        clouds: Clouds {
            dark: 0x3A3660,
            lit: 0xFFC8A5,
            scale: 0.80,
            lo: 0.43,
            hi: 0.58,
            opacity: 1.0,
        },
        water: Water {
            deep: 0x0A1C3A,
            lit: 0x1E5C84,
            sss: 0x5AC8AA,
            sss_k: 0.05,
            foam: 0xD8E2E8,
            foam_k: 1.0,
        },
        swell: System {
            dir_deg: 8.0,
            spread_deg: 18.0,
            octaves: &[(0.95, 125.0), (0.60, 64.0), (0.36, 33.0)],
        },
        wind: System {
            dir_deg: -40.0,
            spread_deg: 34.0,
            octaves: &[(0.20, 17.0), (0.13, 9.6), (0.085, 5.4), (0.048, 3.1)],
        },
    },
    Theme {
        name: "dawn",
        blurb: "Cold light just after sunrise, rippled water, last stars out.",
        eye_h: 18.0,
        horizon_frac: 0.46,
        time_scale: 0.26,
        sky: Sky {
            zenith: 0x14264F,
            mid: 0x7E6EA8,
            horizon: 0xFFC79E,
            haze: 0xFFC49A,
            haze_k: 0.20,
            stars: 0.0030,
        },
        light: Light {
            elev: 0.035,
            ang_r: 0.040,
            intensity: 26.0,
            tint: [1.0, 0.97, 0.93],
            airmass_k: [0.10, 0.22, 0.45],
            aureole: (0.055, 0.60, 0.28, 0.07),
        },
        clouds: Clouds {
            dark: 0x3E4468,
            lit: 0xFFD6C2,
            scale: 0.95,
            lo: 0.52,
            hi: 0.62,
            opacity: 0.85,
        },
        water: Water {
            deep: 0x050A16,
            lit: 0x2A6288,
            sss: 0x8AC0C8,
            sss_k: 0.03,
            foam: 0xE0E8EE,
            foam_k: 0.40,
        },
        swell: System {
            dir_deg: -4.0,
            spread_deg: 10.0,
            octaves: &[(0.55, 140.0), (0.32, 72.0), (0.16, 38.0)],
        },
        wind: System {
            dir_deg: 25.0,
            spread_deg: 20.0,
            octaves: &[(0.110, 15.0), (0.070, 8.0), (0.042, 4.2), (0.024, 2.4)],
        },
    },
    Theme {
        name: "storm",
        blurb: "Grey-green gale. Heavy cover, big steep sea, constant whitecaps.",
        eye_h: 9.0,
        horizon_frac: 0.40,
        time_scale: 0.85,
        sky: Sky {
            zenith: 0x2A2E33,
            mid: 0x4A5157,
            horizon: 0x8A8F86,
            haze: 0xA8A392,
            haze_k: 0.15,
            stars: 0.0,
        },
        light: Light {
            elev: 0.085,
            ang_r: 0.095,
            intensity: 3.4,
            tint: [0.95, 0.95, 0.90],
            airmass_k: [0.35, 0.40, 0.50],
            aureole: (0.230, 0.55, 0.80, 0.20),
        },
        clouds: Clouds {
            dark: 0x1C2024,
            lit: 0x9AA0A0,
            scale: 0.55,
            lo: 0.22,
            hi: 0.52,
            opacity: 1.0,
        },
        water: Water {
            deep: 0x0A1210,
            lit: 0x2E4A42,
            sss: 0x6E9A78,
            sss_k: 0.02,
            foam: 0xE8EEEA,
            foam_k: 2.2,
        },
        swell: System {
            dir_deg: 15.0,
            spread_deg: 22.0,
            octaves: &[(1.60, 95.0), (1.05, 52.0), (0.62, 28.0)],
        },
        wind: System {
            dir_deg: -25.0,
            spread_deg: 40.0,
            octaves: &[(0.42, 15.0), (0.26, 8.5), (0.16, 4.6), (0.09, 2.6)],
        },
    },
    Theme {
        name: "tropical",
        blurb: "Bright daylight, turquoise shallows, white cumulus on deep blue.",
        eye_h: 8.0,
        horizon_frac: 0.52,
        time_scale: 0.45,
        sky: Sky {
            zenith: 0x1A5AC8,
            mid: 0x5AA0E0,
            horizon: 0xBEDCF0,
            haze: 0xFFFFFF,
            haze_k: 0.12,
            stars: 0.0,
        },
        light: Light {
            elev: 0.170,
            ang_r: 0.030,
            intensity: 40.0,
            tint: [1.0, 1.0, 1.0],
            airmass_k: [0.05, 0.09, 0.18],
            aureole: (0.030, 0.35, 0.15, 0.04),
        },
        clouds: Clouds {
            dark: 0x8090A8,
            lit: 0xFFFFFF,
            scale: 0.90,
            lo: 0.48,
            hi: 0.62,
            opacity: 0.95,
        },
        water: Water {
            deep: 0x043C5A,
            lit: 0x18A0B4,
            sss: 0x40E0C0,
            sss_k: 0.08,
            foam: 0xFFFFFF,
            foam_k: 1.4,
        },
        swell: System {
            dir_deg: -12.0,
            spread_deg: 16.0,
            octaves: &[(0.60, 80.0), (0.36, 42.0), (0.22, 22.0)],
        },
        wind: System {
            dir_deg: 30.0,
            spread_deg: 30.0,
            octaves: &[(0.14, 14.0), (0.09, 7.5), (0.055, 4.0), (0.030, 2.2)],
        },
    },
    Theme {
        name: "night",
        blurb: "Moon low over black water, silver road, full star field.",
        eye_h: 14.0,
        horizon_frac: 0.42,
        time_scale: 0.28,
        sky: Sky {
            zenith: 0x050A1E,
            mid: 0x0E1838,
            horizon: 0x27354F,
            haze: 0x3A4A6A,
            haze_k: 0.15,
            stars: 0.0085,
        },
        light: Light {
            elev: 0.110,
            ang_r: 0.020,
            intensity: 3.2,
            tint: [0.86, 0.92, 1.0],
            airmass_k: [0.10, 0.12, 0.16],
            aureole: (0.050, 0.30, 0.28, 0.05),
        },
        clouds: Clouds {
            dark: 0x0A1024,
            lit: 0x66708E,
            scale: 0.70,
            lo: 0.50,
            hi: 0.68,
            opacity: 0.90,
        },
        water: Water {
            deep: 0x02060E,
            lit: 0x0C1E38,
            sss: 0x203A5A,
            sss_k: 0.01,
            foam: 0x9AAEC8,
            foam_k: 0.80,
        },
        swell: System {
            dir_deg: 5.0,
            spread_deg: 14.0,
            octaves: &[(0.80, 115.0), (0.50, 60.0), (0.30, 31.0)],
        },
        wind: System {
            dir_deg: -35.0,
            spread_deg: 30.0,
            octaves: &[(0.13, 16.0), (0.08, 9.0), (0.05, 5.0)],
        },
    },
    Theme {
        name: "ember",
        blurb: "Sun on the horizon through heavy dust. Deep red, near-black sea.",
        eye_h: 12.0,
        horizon_frac: 0.46,
        time_scale: 0.30,
        sky: Sky {
            zenith: 0x2A0E2E,
            mid: 0x8A2438,
            horizon: 0xE8621E,
            haze: 0xFF7A20,
            haze_k: 0.34,
            stars: 0.0,
        },
        light: Light {
            elev: 0.030,
            ang_r: 0.050,
            intensity: 34.0,
            tint: [1.0, 1.0, 1.0],
            airmass_k: [0.10, 0.42, 0.95],
            aureole: (0.060, 0.70, 0.30, 0.14),
        },
        clouds: Clouds {
            dark: 0x2E1420,
            lit: 0xFF9A4A,
            scale: 0.65,
            lo: 0.40,
            hi: 0.56,
            opacity: 1.0,
        },
        water: Water {
            deep: 0x100608,
            lit: 0x3A1A18,
            sss: 0xE07038,
            sss_k: 0.07,
            foam: 0xE8C0A0,
            foam_k: 0.80,
        },
        swell: System {
            dir_deg: -6.0,
            spread_deg: 15.0,
            octaves: &[(0.90, 118.0), (0.56, 61.0), (0.34, 32.0)],
        },
        wind: System {
            dir_deg: 38.0,
            spread_deg: 32.0,
            octaves: &[(0.18, 16.0), (0.11, 9.0), (0.07, 5.0), (0.04, 2.9)],
        },
    },
    Theme {
        name: "arctic",
        blurb: "Cold low sun on flat calm. Ice-blue water, very wide.",
        eye_h: 20.0,
        horizon_frac: 0.50,
        time_scale: 0.18,
        sky: Sky {
            zenith: 0x14468C,
            mid: 0x5C9EC8,
            horizon: 0xC8E8F2,
            haze: 0xDCF2FF,
            haze_k: 0.16,
            stars: 0.0,
        },
        light: Light {
            elev: 0.045,
            ang_r: 0.038,
            intensity: 16.0,
            tint: [0.86, 0.94, 1.0],
            airmass_k: [0.06, 0.07, 0.10],
            aureole: (0.070, 0.40, 0.32, 0.07),
        },
        clouds: Clouds {
            dark: 0x7089A8,
            lit: 0xF0FAFF,
            scale: 1.10,
            lo: 0.54,
            hi: 0.60,
            opacity: 0.68,
        },
        water: Water {
            deep: 0x04121F,
            lit: 0x1E9AB8,
            sss: 0xA8E8F0,
            sss_k: 0.03,
            foam: 0xFFFFFF,
            foam_k: 0.50,
        },
        swell: System {
            dir_deg: 2.0,
            spread_deg: 8.0,
            octaves: &[(0.42, 150.0), (0.24, 78.0), (0.12, 40.0)],
        },
        wind: System {
            dir_deg: -20.0,
            spread_deg: 18.0,
            octaves: &[(0.040, 14.0), (0.022, 7.0), (0.012, 3.6)],
        },
    },
    Theme {
        name: "mist",
        blurb: "Cold sea haar. The sun a pale smear, the water gone glassy grey.",
        eye_h: 7.0,
        horizon_frac: 0.47,
        time_scale: 0.22,
        sky: Sky {
            zenith: 0x8C9AA6,
            mid: 0xB4BEC4,
            horizon: 0xE2E6E4,
            haze: 0xF2F0EA,
            haze_k: 0.55,
            stars: 0.0,
        },
        light: Light {
            elev: 0.075,
            ang_r: 0.070,
            intensity: 3.0,
            tint: [1.0, 0.99, 0.96],
            airmass_k: [0.30, 0.33, 0.38],
            aureole: (0.220, 0.75, 1.10, 0.35),
        },
        clouds: Clouds {
            dark: 0xA8B0B4,
            lit: 0xF4F4F0,
            scale: 0.45,
            lo: 0.20,
            hi: 0.75,
            opacity: 0.50,
        },
        water: Water {
            deep: 0x121A1C,
            lit: 0x3E565C,
            sss: 0x6E8A88,
            sss_k: 0.02,
            foam: 0xDCE4E2,
            foam_k: 0.25,
        },
        swell: System {
            dir_deg: 6.0,
            spread_deg: 10.0,
            octaves: &[(0.50, 135.0), (0.30, 70.0), (0.16, 36.0)],
        },
        wind: System {
            dir_deg: -18.0,
            spread_deg: 22.0,
            octaves: &[(0.055, 15.0), (0.032, 8.2), (0.018, 4.4)],
        },
    },
    // The only theme with the light above ten degrees. The top of frame sits at
    // atan(horizon_frac), so 0.46 leaves the disc at about 12% down, clear of
    // the horizon and of the cloud deck. The glitter is still a road rather
    // than a compact patch, because the wind octaves tilt facets toward the sun
    // across the whole near field, but it broadens and brightens toward the
    // viewer instead of narrowing to a point at the horizon the way every
    // low-sun theme does. None of that costs rendering code: the reflected ray
    // reads the same sky function the sky pixels do.
    Theme {
        name: "noon",
        blurb: "Sun high and hard. Deep blue open ocean under sharp-edged cumulus.",
        eye_h: 12.0,
        horizon_frac: 0.46,
        time_scale: 0.38,
        sky: Sky {
            zenith: 0x0E3EA8,
            mid: 0x3F86D8,
            horizon: 0xAECBE8,
            haze: 0xF2F8FF,
            // A high sun leaves almost nothing piled at the horizon.
            haze_k: 0.08,
            stars: 0.0,
        },
        light: Light {
            elev: 0.330,
            ang_r: 0.026,
            intensity: 46.0,
            tint: [1.0, 1.0, 1.0],
            // Flat and low. Airmass is 2.2 here against tropical's 3.5, so the
            // disc comes out white on its own and nothing is picked by hand.
            airmass_k: [0.040, 0.050, 0.070],
            aureole: (0.022, 0.28, 0.11, 0.025),
        },
        clouds: Clouds {
            dark: 0x6E86A8,
            lit: 0xFFFFFF,
            scale: 1.05,
            // Narrower window than tropical, which is what hardens the edges.
            lo: 0.52,
            hi: 0.60,
            opacity: 1.0,
        },
        water: Water {
            deep: 0x02203E,
            lit: 0x0F6EA8,
            // Deep ocean, so almost nothing comes through a crest. Tropical
            // runs 0x40E0C0 at 0.08 and wants to look shallow. Anything near
            // that here turned the foreground into a reef flat.
            sss: 0x1C7FA0,
            sss_k: 0.030,
            foam: 0xFFFFFF,
            foam_k: 1.1,
        },
        swell: System {
            dir_deg: 8.0,
            spread_deg: 14.0,
            octaves: &[(0.70, 95.0), (0.40, 50.0), (0.24, 26.0)],
        },
        wind: System {
            dir_deg: -24.0,
            spread_deg: 28.0,
            octaves: &[(0.12, 13.0), (0.075, 7.0), (0.045, 3.8), (0.025, 2.1)],
        },
    },
    // The only theme with the light below the horizon, which is the floor of
    // the same relation `noon` found the ceiling of. The horizon row sits at
    // elevation 0, so a disc of angular radius r stops being drawn once elev
    // is past -1.2r. At -0.055 against r 0.035 it is gone, and the aureole is
    // all that is left of the sun. That makes the aureole load bearing rather
    // than decorative: the first pass ran the wide lobe at (0.40, 0.16) and it
    // painted red across the whole sky, so the twilight came out violet like
    // `dusk` instead of teal. Narrow lobes keep the warmth at the horizon.
    // `extinction` clamps `sin_elev` at 0.01, so nothing below about elev 0.01
    // reddens any further; the colour here is that clamp, not the geometry.
    Theme {
        name: "gloaming",
        blurb: "Sun a few minutes gone. Green-blue afterglow, first stars, black water.",
        eye_h: 16.0,
        horizon_frac: 0.45,
        time_scale: 0.24,
        sky: Sky {
            zenith: 0x0C1638,
            mid: 0x1E4E7A,
            horizon: 0x59A8A0,
            haze: 0xB4562E,
            haze_k: 0.22,
            // A tenth of `night`, because stars reflect off the water as hard
            // single pixels and a sky this bright cannot hide them.
            stars: 0.0006,
        },
        light: Light {
            elev: -0.055,
            ang_r: 0.035,
            intensity: 14.0,
            tint: [1.0, 1.0, 1.0],
            airmass_k: [0.16, 0.60, 1.20],
            aureole: (0.045, 0.45, 0.16, 0.06),
        },
        clouds: Clouds {
            dark: 0x1A2440,
            lit: 0x8A6A78,
            scale: 0.75,
            lo: 0.55,
            hi: 0.70,
            opacity: 0.75,
        },
        water: Water {
            deep: 0x03060F,
            lit: 0x0E2440,
            sss: 0x2A5A66,
            sss_k: 0.015,
            foam: 0xA8BAC8,
            foam_k: 0.55,
        },
        swell: System {
            dir_deg: 4.0,
            spread_deg: 12.0,
            octaves: &[(0.70, 130.0), (0.42, 68.0), (0.22, 35.0)],
        },
        wind: System {
            dir_deg: -28.0,
            spread_deg: 24.0,
            octaves: &[(0.075, 15.0), (0.045, 8.4), (0.026, 4.5), (0.014, 2.5)],
        },
    },
    // The lowest eye in the set at 4 m, against `arctic`'s 20. That is a
    // distance knob and nothing else: `render()` takes z from the row and
    // `eye_h` alone, on flat water, so wave height never moves the geometry and
    // no crest can occlude anything behind it. What a low eye actually does is
    // put the horizon at 1440 m instead of 7200, which magnifies whatever the
    // spectrum contains. The wavelengths below are scaled to match, roughly a
    // third of `dusk`'s. The first pass kept ordinary 70 m swell and 12 m chop
    // and rendered a sheet of glass, because from 4 m up a 12 m wave spans most
    // of the frame. Halve the eye and you have to halve the sea with it.
    //
    // `horizon_frac` 0.36 is the lowest in the set, below `storm`'s 0.40, and
    // it is a composition choice rather than a physical one. At 0.42 the round
    // crop in the gallery centred on the horizon and the theme read as the
    // palest card there, since the jade is all below the water line. Lifting
    // the horizon puts water through the middle of the circle.
    Theme {
        name: "trough",
        blurb: "Down in the swell at four metres. Jade water under a low broken deck.",
        eye_h: 4.0,
        horizon_frac: 0.36,
        time_scale: 0.55,
        sky: Sky {
            zenith: 0x184068,
            mid: 0x5A7C96,
            horizon: 0xA8B8B2,
            haze: 0xD8DCC8,
            haze_k: 0.14,
            stars: 0.0,
        },
        light: Light {
            elev: 0.120,
            ang_r: 0.055,
            intensity: 10.0,
            tint: [1.0, 0.99, 0.94],
            airmass_k: [0.20, 0.26, 0.36],
            aureole: (0.100, 0.50, 0.32, 0.08),
        },
        clouds: Clouds {
            dark: 0x2C3C54,
            lit: 0xCED6CE,
            scale: 0.80,
            lo: 0.46,
            hi: 0.56,
            opacity: 0.95,
        },
        water: Water {
            deep: 0x031A16,
            lit: 0x1E8C6C,
            sss: 0x58C89A,
            sss_k: 0.045,
            foam: 0xE8F0EA,
            foam_k: 1.5,
        },
        swell: System {
            dir_deg: 10.0,
            spread_deg: 20.0,
            octaves: &[(0.35, 26.0), (0.20, 14.0), (0.12, 7.5)],
        },
        wind: System {
            dir_deg: -32.0,
            spread_deg: 34.0,
            octaves: &[(0.060, 4.5), (0.038, 2.6), (0.022, 1.5), (0.012, 0.85)],
        },
    },
];
