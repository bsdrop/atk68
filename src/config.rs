//! Local `key = value` profiles for keyboard settings.

use crate::actuation::{self, Actuation};
use crate::device::{Keyboard, REPORT_RATE_HZ};
use crate::light::{self, Light};
use crate::tweaks;
use anyhow::{Context, Result};

pub struct Config {
    pub light: Light,
    pub act: Actuation,
    pub rate_index: u8,
    pub step: f32,
    pub tweaks: Vec<u8>,
    pub tweak_set: Vec<bool>,
}

impl Config {
    pub fn from_device(kb: &Keyboard) -> Result<Config> {
        let mut cfg = Self::base_from_device(kb)?;
        for (i, t) in tweaks::TOGGLES.iter().enumerate() {
            cfg.tweaks[i] = tweaks::get(kb, t)?;
            cfg.tweak_set[i] = true;
        }
        Ok(cfg)
    }

    // Toggle values are sent only when explicitly present in an apply file.
    fn base_from_device(kb: &Keyboard) -> Result<Config> {
        Ok(Config {
            light: light::get(kb)?,
            act: actuation::get(kb)?,
            rate_index: kb.report_rate()?,
            step: kb.travel_step,
            tweaks: vec![0; tweaks::TOGGLES.len()],
            tweak_set: vec![false; tweaks::TOGGLES.len()],
        })
    }

    pub fn serialize(&self) -> String {
        let l = &self.light;
        let a = &self.act;
        let hz = REPORT_RATE_HZ
            .get(self.rate_index as usize)
            .copied()
            .unwrap_or(0);
        let mut out = format!(
            "# atk68 profile — apply with: atk68 apply <file>\n\
             light.enabled = {}\n\
             light.effect = {}\n\
             light.brightness = {}\n\
             light.speed = {}\n\
             light.color = #{:02X}{:02X}{:02X}\n\
             light.rainbow = {}\n\
             actuation.point_mm = {:.1}\n\
             actuation.press_mm = {:.1}\n\
             actuation.release_mm = {:.1}\n\
             rate.hz = {}\n",
            l.enabled,
            l.mode,
            l.brightness,
            l.speed,
            l.rgb.0,
            l.rgb.1,
            l.rgb.2,
            l.colorful,
            actuation::mm(a.point, self.step),
            actuation::mm(a.down, self.step),
            actuation::mm(a.up, self.step),
            hz,
        );
        for (t, &value) in tweaks::TOGGLES.iter().zip(&self.tweaks) {
            let value = match value {
                0 => "off".to_string(),
                1 => "on".to_string(),
                n => n.to_string(),
            };
            out.push_str(&format!("tweak.{} = {value}\n", t.key));
        }
        out
    }

    /// Override fields from a config file's text (file values win).
    pub fn apply_overrides(&mut self, text: &str) -> Result<()> {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue; // full-line comment (don't split on '#': colors use it)
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let (k, v) = (k.trim(), v.trim());
            let tweak_key = k.strip_prefix("tweak.").unwrap_or(k);
            if let Some(i) = tweaks::TOGGLES.iter().position(|t| t.key == tweak_key) {
                self.tweaks[i] = tweaks::parse_value(v)?;
                self.tweak_set[i] = true;
                continue;
            }
            match k {
                "light.enabled" => self.light.enabled = parse_bool(v)?,
                "light.effect" => self.light.mode = v.parse()?,
                "light.brightness" => self.light.brightness = v.parse()?,
                "light.speed" => self.light.speed = v.parse()?,
                "light.color" => self.light.rgb = light::parse_rgb(v)?,
                "light.rainbow" => self.light.colorful = parse_bool(v)?,
                "actuation.point_mm" => self.act.point = actuation::raw(v.parse()?, self.step),
                "actuation.press_mm" => self.act.down = actuation::raw(v.parse()?, self.step),
                "actuation.release_mm" => self.act.up = actuation::raw(v.parse()?, self.step),
                "rate.hz" => self.rate_index = nearest_rate(v.parse()?),
                _ => {}
            }
        }
        Ok(())
    }

    /// Push the whole config to the keyboard (RAM). `rate` is applied last and
    /// only if changed, since a polling-rate change can re-enumerate the device.
    pub fn apply(&self, kb: &Keyboard) -> Result<()> {
        light::set(kb, &self.light)?;
        actuation::set(kb, &self.act)?;
        for ((t, &value), &set) in tweaks::TOGGLES
            .iter()
            .zip(&self.tweaks)
            .zip(&self.tweak_set)
        {
            if set {
                tweaks::set(kb, t, value)?;
            }
        }
        if kb.report_rate()? != self.rate_index {
            kb.set_report_rate(self.rate_index)?;
        }
        Ok(())
    }
}

pub fn export(kb: &Keyboard, path: &str) -> Result<()> {
    let cfg = Config::from_device(kb)?;
    std::fs::write(path, cfg.serialize()).with_context(|| format!("write {path}"))?;
    Ok(())
}

pub fn apply_file(kb: &Keyboard, path: &str) -> Result<()> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {path}"))?;
    let mut cfg = Config::base_from_device(kb)?; // base = current, file overrides
    cfg.apply_overrides(&text)?;
    cfg.apply(kb)
}

fn parse_bool(v: &str) -> Result<bool> {
    Ok(matches!(
        v.to_lowercase().as_str(),
        "true" | "1" | "on" | "yes"
    ))
}

fn nearest_rate(hz: u32) -> u8 {
    REPORT_RATE_HZ
        .iter()
        .enumerate()
        .min_by_key(|(_, h)| h.abs_diff(hz))
        .map(|(i, _)| i as u8)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tweak_overrides() {
        let mut cfg = Config {
            light: Light {
                enabled: false,
                mode: 0,
                brightness: 0,
                speed: 0,
                rgb: (0, 0, 0),
                colorful: false,
            },
            act: Actuation {
                point: 1,
                down: 1,
                up: 1,
            },
            rate_index: 0,
            step: 0.1,
            tweaks: vec![0; tweaks::TOGGLES.len()],
            tweak_set: vec![false; tweaks::TOGGLES.len()],
        };

        cfg.apply_overrides("tweak.rt = on\nlow-latency = 1\n")
            .unwrap();
        let rt = tweaks::TOGGLES.iter().position(|t| t.key == "rt").unwrap();
        let low = tweaks::TOGGLES
            .iter()
            .position(|t| t.key == "low-latency")
            .unwrap();
        assert_eq!(cfg.tweaks[rt], 1);
        assert_eq!(cfg.tweaks[low], 1);
        assert!(cfg.tweak_set[rt] && cfg.tweak_set[low]);
    }
}
