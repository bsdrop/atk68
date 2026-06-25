//! Local settings profile: export the keyboard's current state to a plain text
//! file, and re-apply it in one shot. This makes the keyboard's occasional flash
//! wipe a non-issue — `atk68 apply my.conf` restores everything offline.
//!
//! Format is a dependency-free `key = value` list; unknown keys are ignored and
//! missing keys keep the keyboard's current value, so partial files are fine.

use crate::actuation::{self, Actuation};
use crate::device::{Keyboard, REPORT_RATE_HZ};
use crate::light::{self, Light};
use anyhow::{Context, Result};

pub struct Config {
    pub light: Light,
    pub act: Actuation,
    pub rate_index: u8,
    pub step: f32,
}

impl Config {
    pub fn from_device(kb: &Keyboard) -> Result<Config> {
        Ok(Config {
            light: light::get(kb)?,
            act: actuation::get(kb)?,
            rate_index: kb.report_rate()?,
            step: kb.travel_step,
        })
    }

    pub fn serialize(&self) -> String {
        let l = &self.light;
        let a = &self.act;
        let hz = REPORT_RATE_HZ.get(self.rate_index as usize).copied().unwrap_or(0);
        format!(
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
            l.enabled, l.mode, l.brightness, l.speed,
            l.rgb.0, l.rgb.1, l.rgb.2, l.colorful,
            actuation::mm(a.point, self.step), actuation::mm(a.down, self.step),
            actuation::mm(a.up, self.step), hz,
        )
    }

    /// Override fields from a config file's text (file values win).
    pub fn apply_overrides(&mut self, text: &str) -> Result<()> {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue; // full-line comment (don't split on '#': colors use it)
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
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
    let mut cfg = Config::from_device(kb)?; // base = current, file overrides
    cfg.apply_overrides(&text)?;
    cfg.apply(kb)
}

fn parse_bool(v: &str) -> Result<bool> {
    Ok(matches!(v.to_lowercase().as_str(), "true" | "1" | "on" | "yes"))
}

fn nearest_rate(hz: u32) -> u8 {
    REPORT_RATE_HZ
        .iter()
        .enumerate()
        .min_by_key(|(_, h)| h.abs_diff(hz))
        .map(|(i, _)| i as u8)
        .unwrap_or(0)
}
