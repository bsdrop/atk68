//! Local `key = value` profiles. Parsing is independent of the keyboard.

use crate::actuation::{self, Actuation};
use crate::device::{self, Keyboard, REPORT_RATE_HZ};
use crate::light::{self, Light};
use crate::tweaks::{self, ToggleId};
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::fmt::Display;

/// Only fields present in a profile are written to the keyboard.
#[derive(Default, Debug, PartialEq)]
pub struct Profile {
    light: LightPatch,
    actuation: ActuationPatch,
    rate_hz: Option<u32>,
    tweaks: BTreeMap<ToggleId, u8>,
}

#[derive(Default, Debug, PartialEq)]
struct LightPatch {
    enabled: Option<bool>,
    effect: Option<u8>,
    brightness: Option<u8>,
    speed: Option<u8>,
    color: Option<(u8, u8, u8)>,
    rainbow: Option<bool>,
}

impl LightPatch {
    fn is_empty(&self) -> bool {
        self.enabled.is_none()
            && self.effect.is_none()
            && self.brightness.is_none()
            && self.speed.is_none()
            && self.color.is_none()
            && self.rainbow.is_none()
    }

    fn apply(&self, light: &mut Light) {
        if let Some(value) = self.enabled {
            light.enabled = value;
        }
        if let Some(value) = self.effect {
            light.mode = value;
        }
        if let Some(value) = self.brightness {
            light.brightness = value;
        }
        if let Some(value) = self.speed {
            light.speed = value;
        }
        if let Some(value) = self.color {
            light.rgb = value;
        }
        if let Some(value) = self.rainbow {
            light.colorful = value;
        }
    }
}

#[derive(Default, Debug, PartialEq)]
struct ActuationPatch {
    point_mm: Option<f32>,
    press_mm: Option<f32>,
    release_mm: Option<f32>,
}

impl ActuationPatch {
    fn is_empty(&self) -> bool {
        self.point_mm.is_none() && self.press_mm.is_none() && self.release_mm.is_none()
    }

    fn apply(&self, actuation: &mut Actuation, step: f32) {
        if let Some(value) = self.point_mm {
            actuation.point = actuation::raw(value, step);
        }
        if let Some(value) = self.press_mm {
            actuation.down = actuation::raw(value, step);
        }
        if let Some(value) = self.release_mm {
            actuation.up = actuation::raw(value, step);
        }
    }
}

impl Profile {
    pub fn parse(text: &str) -> Result<Self> {
        let mut profile = Self::default();
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue; // Color values may contain '#', so only full-line comments are skipped.
            }
            profile
                .parse_line(line)
                .with_context(|| format!("profile line {}: {line}", index + 1))?;
        }
        Ok(profile)
    }

    fn parse_line(&mut self, line: &str) -> Result<()> {
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("expected key = value"))?;
        let (key, value) = (key.trim(), value.trim());
        let tweak_key = key.strip_prefix("tweak.").unwrap_or(key);
        if let Some(toggle) = tweaks::find(tweak_key) {
            self.tweaks.insert(toggle.id, tweaks::parse_value(value)?);
            return Ok(());
        }
        match key {
            "light.enabled" => self.light.enabled = Some(parse_bool(value)?),
            "light.effect" => self.light.effect = Some(value.parse()?),
            "light.brightness" => {
                self.light.brightness = Some(light::validate_brightness(value.parse()?)?)
            }
            "light.speed" => self.light.speed = Some(light::validate_speed(value.parse()?)?),
            "light.color" => self.light.color = Some(light::parse_rgb(value)?),
            "light.rainbow" => self.light.rainbow = Some(parse_bool(value)?),
            "actuation.point_mm" => self.actuation.point_mm = Some(parse_mm(value)?),
            "actuation.press_mm" => self.actuation.press_mm = Some(parse_mm(value)?),
            "actuation.release_mm" => self.actuation.release_mm = Some(parse_mm(value)?),
            "rate.hz" => self.rate_hz = Some(value.parse()?),
            _ => bail!("unknown setting '{key}'"),
        }
        Ok(())
    }

    pub fn from_device(kb: &Keyboard) -> Result<Self> {
        let light = light::get(kb)?;
        let act = actuation::get(kb)?;
        let index = kb.report_rate()? as usize;
        let rate_hz = *REPORT_RATE_HZ
            .get(index)
            .with_context(|| format!("unknown report-rate index {index}"))?;
        let mut tweaks = BTreeMap::new();
        for toggle in tweaks::TOGGLES {
            tweaks.insert(toggle.id, tweaks::get(kb, toggle)?);
        }
        Ok(Self {
            light: LightPatch {
                enabled: Some(light.enabled),
                effect: Some(light.mode),
                brightness: Some(light.brightness),
                speed: Some(light.speed),
                color: Some(light.rgb),
                rainbow: Some(light.colorful),
            },
            actuation: ActuationPatch {
                point_mm: Some(actuation::mm(act.point, kb.travel_step)),
                press_mm: Some(actuation::mm(act.down, kb.travel_step)),
                release_mm: Some(actuation::mm(act.up, kb.travel_step)),
            },
            rate_hz: Some(rate_hz),
            tweaks,
        })
    }

    pub fn serialize(&self) -> String {
        let mut out = String::from("# atk68 profile — apply with: atk68 apply <file>\n");
        if let Some(value) = self.light.enabled {
            push_line(&mut out, "light.enabled", value);
        }
        if let Some(value) = self.light.effect {
            push_line(&mut out, "light.effect", value);
        }
        if let Some(value) = self.light.brightness {
            push_line(&mut out, "light.brightness", value);
        }
        if let Some(value) = self.light.speed {
            push_line(&mut out, "light.speed", value);
        }
        if let Some((r, g, b)) = self.light.color {
            push_line(&mut out, "light.color", format!("#{r:02X}{g:02X}{b:02X}"));
        }
        if let Some(value) = self.light.rainbow {
            push_line(&mut out, "light.rainbow", value);
        }
        for (key, value) in [
            ("actuation.point_mm", self.actuation.point_mm),
            ("actuation.press_mm", self.actuation.press_mm),
            ("actuation.release_mm", self.actuation.release_mm),
        ] {
            if let Some(value) = value {
                push_line(&mut out, key, format!("{value:.2}"));
            }
        }
        if let Some(value) = self.rate_hz {
            push_line(&mut out, "rate.hz", value);
        }
        for toggle in tweaks::TOGGLES {
            if let Some(value) = self.tweaks.get(&toggle.id) {
                let shown = match value {
                    0 => "off".to_string(),
                    1 => "on".to_string(),
                    other => other.to_string(),
                };
                push_line(&mut out, &format!("tweak.{}", toggle.key), shown);
            }
        }
        out
    }

    /// Apply only named sections. Rate is last because changing it may re-enumerate USB.
    pub fn apply(&self, kb: &Keyboard) -> Result<()> {
        if !self.light.is_empty() {
            let mut light = light::get(kb)?;
            self.light.apply(&mut light);
            light::set(kb, &light)?;
        }
        if !self.actuation.is_empty() {
            let mut actuation = actuation::get(kb)?;
            self.actuation.apply(&mut actuation, kb.travel_step);
            actuation::set(kb, &actuation)?;
        }
        for toggle in tweaks::TOGGLES {
            if let Some(value) = self.tweaks.get(&toggle.id) {
                tweaks::set(kb, toggle, *value)?;
            }
        }
        if let Some(hz) = self.rate_hz {
            let index = device::nearest_rate(hz);
            if kb.report_rate()? != index {
                kb.set_report_rate(index)?;
            }
        }
        Ok(())
    }
}

pub fn export(kb: &Keyboard, path: &str) -> Result<()> {
    let profile = Profile::from_device(kb)?;
    std::fs::write(path, profile.serialize()).with_context(|| format!("write {path}"))?;
    Ok(())
}

pub fn apply_file(path: &str) -> Result<()> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {path}"))?;
    let profile = Profile::parse(&text)?;
    profile.apply(&Keyboard::open()?)
}

fn push_line(out: &mut String, key: &str, value: impl Display) {
    out.push_str(&format!("{key} = {value}\n"));
}

fn parse_bool(value: &str) -> Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "on" | "yes" => Ok(true),
        "false" | "0" | "off" | "no" => Ok(false),
        _ => bail!("invalid boolean '{value}' (use true/false, on/off, yes/no, or 1/0)"),
    }
}

fn parse_mm(value: &str) -> Result<f32> {
    let mm: f32 = value.parse()?;
    if !mm.is_finite() || mm <= 0.0 || mm > 4.0 {
        bail!("actuation must be greater than 0 and at most 4.0 mm");
    }
    Ok(mm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_toggles_and_boolean_spellings() {
        let profile = Profile::parse(
            "light.enabled = YES\nlight.rainbow = 0\ntweak.rt = on\nlow-latency = 1\n",
        )
        .unwrap();
        assert_eq!(profile.light.enabled, Some(true));
        assert_eq!(profile.light.rainbow, Some(false));
        assert_eq!(profile.tweaks.get(&ToggleId::Rt), Some(&1));
        assert_eq!(profile.tweaks.get(&ToggleId::LowLatency), Some(&1));
    }

    #[test]
    fn rejects_misspelled_boolean_with_line_number() {
        let error = Profile::parse("light.rainbow = true\nlight.enabled = ture\n").unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("profile line 2: light.enabled = ture"));
        assert!(message.contains("invalid boolean 'ture'"));
    }

    #[test]
    fn rejects_malformed_and_unknown_settings() {
        for line in ["light.enabled true", "light.enabld = true"] {
            let error = Profile::parse(line).unwrap_err();
            assert!(format!("{error:#}").contains("profile line 1"));
        }
    }

    #[test]
    fn partial_profile_contains_only_named_sections() {
        let profile = Profile::parse("tweak.rt = on").unwrap();
        assert!(profile.light.is_empty());
        assert!(profile.actuation.is_empty());
        assert_eq!(profile.rate_hz, None);
        assert_eq!(
            profile.serialize(),
            "# atk68 profile — apply with: atk68 apply <file>\ntweak.rt = on\n"
        );
    }

    #[test]
    fn fine_actuation_value_round_trips() {
        let profile = Profile::parse("actuation.point_mm = 0.02\n").unwrap();
        let text = profile.serialize();
        assert!(text.contains("actuation.point_mm = 0.02\n"));
        assert_eq!(Profile::parse(&text).unwrap(), profile);
    }

    #[test]
    fn rejects_non_finite_actuation() {
        assert!(Profile::parse("actuation.point_mm = NaN").is_err());
    }

    #[test]
    fn rejects_out_of_range_lighting() {
        assert!(Profile::parse("light.brightness = 9").is_err());
        assert!(Profile::parse("light.speed = 5").is_err());
    }
}
