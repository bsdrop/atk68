//! RGB lighting model for the ATK68. Field layout verified by write-then-read
//! round-trips on real hardware.
//!
//! GET 0x45 reply data: `[enabled, mode, brightness, speed, R, G, B, colorful]`
//! SET 0x44 body:       `[len=8, enabled, mode, brightness, speed, R, G, B, 0, colorful]`

use crate::device::{op, Keyboard, REPORT_LEN};
use anyhow::{bail, Result};

#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub enabled: bool,
    pub mode: u8,            // effect id (see i18n::EFFECTS)
    pub brightness: u8,      // 0..=BRIGHTNESS_MAX
    pub speed: u8,           // 0..=SPEED_MAX
    pub rgb: (u8, u8, u8),
    pub colorful: bool,      // rainbow / per-LED multicolour flag
}

pub const BRIGHTNESS_MAX: u8 = 8;
pub const SPEED_MAX: u8 = 4;

impl Light {
    fn decode(d: &[u8]) -> Result<Light> {
        if d.len() < 8 {
            bail!("short light reply ({} bytes)", d.len());
        }
        Ok(Light {
            enabled: d[0] != 0,
            mode: d[1],
            brightness: d[2],
            speed: d[3],
            rgb: (d[4], d[5], d[6]),
            colorful: d[7] != 0,
        })
    }

    fn body(&self) -> [u8; 10] {
        [
            8,                    // length
            self.enabled as u8,
            self.mode,
            self.brightness,
            self.speed,
            self.rgb.0,
            self.rgb.1,
            self.rgb.2,
            0,
            self.colorful as u8,
        ]
    }
}

/// Read the current main-zone lighting (light type 0).
pub fn get(kb: &Keyboard) -> Result<Light> {
    let r = kb.exchange(op::GET_LIGHT, &[0, 0])?;
    if r[2] != 0 {
        bail!("keyboard returned error 0x{:02x} reading lighting", r[2]);
    }
    Light::decode(Keyboard::data(&r))
}

/// Apply lighting to RAM (reverts on replug; this tool never writes flash).
pub fn set(kb: &Keyboard, l: &Light) -> Result<()> {
    let _: [u8; REPORT_LEN] = kb.exchange(op::SET_LIGHT, &l.body())?;
    Ok(())
}

/// Parse "#RRGGBB" or "r,g,b" into an RGB triple.
pub fn parse_rgb(s: &str) -> Result<(u8, u8, u8)> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() != 6 {
            bail!("hex color must be #RRGGBB");
        }
        let v = u32::from_str_radix(hex, 16)?;
        return Ok(((v >> 16) as u8, (v >> 8) as u8, v as u8));
    }
    let p: Vec<&str> = s.split(',').collect();
    if p.len() == 3 {
        return Ok((p[0].trim().parse()?, p[1].trim().parse()?, p[2].trim().parse()?));
    }
    bail!("color must be #RRGGBB or r,g,b")
}
