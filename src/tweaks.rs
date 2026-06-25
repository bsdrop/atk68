//! Simple single-byte keyboard settings (gtech). Each is `GET [0,0] -> value`
//! and `SET [1, value]`, so they share one tiny implementation.

use crate::device::Keyboard;
use anyhow::{bail, Result};

pub struct Toggle {
    pub key: &'static str,
    pub get: u8,
    pub set: u8,
    pub help: &'static str,
}

pub const TOGGLES: &[Toggle] = &[
    Toggle { key: "winlock", get: 0x1D, set: 0x1E, help: "disable the Windows key" },
    Toggle { key: "wasd", get: 0x19, set: 0x1A, help: "swap WASD with arrow keys" },
    Toggle { key: "mac", get: 0x1B, set: 0x1C, help: "Mac layout (0=Win, 1=Mac)" },
    Toggle { key: "rt", get: 0x5D, set: 0x5E, help: "global rapid trigger" },
    Toggle { key: "low-latency", get: 0x27, set: 0x28, help: "low-latency mode" },
    Toggle { key: "bottom-trigger", get: 0x29, set: 0x2A, help: "full-press bottom trigger" },
    Toggle { key: "debounce", get: 0x25, set: 0x26, help: "key debounce (raw units)" },
];

pub fn find(key: &str) -> Option<&'static Toggle> {
    TOGGLES.iter().find(|t| t.key == key)
}

pub fn get(kb: &Keyboard, t: &Toggle) -> Result<u8> {
    let r = kb.exchange(t.get, &[0, 0])?;
    Ok(Keyboard::data(&r).first().copied().unwrap_or(0))
}

pub fn set(kb: &Keyboard, t: &Toggle, value: u8) -> Result<()> {
    kb.exchange(t.set, &[1, value])?;
    Ok(())
}

/// Parse a value like `on`/`off`/`1`/`0`/`3`.
pub fn parse_value(v: &str) -> Result<u8> {
    match v.to_lowercase().as_str() {
        "on" | "true" | "yes" => Ok(1),
        "off" | "false" | "no" => Ok(0),
        n => n.parse().map_err(|_| anyhow::anyhow!("bad value '{v}' (on/off/number)")),
    }
}

/// Apply `key=value` pairs, then print every toggle's current state.
pub fn run(kb: &Keyboard, sets: &[String]) -> Result<()> {
    for pair in sets {
        let (k, v) = pair.split_once('=').ok_or_else(|| anyhow::anyhow!("use name=value"))?;
        let Some(t) = find(k.trim()) else { bail!("unknown setting '{k}'") };
        set(kb, t, parse_value(v.trim())?)?;
    }
    for t in TOGGLES {
        let v = get(kb, t)?;
        let shown = match v {
            0 => "off".to_string(),
            1 => "on".to_string(),
            n => n.to_string(),
        };
        println!("  {:<15} {:<4}  ({})", t.key, shown, t.help);
    }
    Ok(())
}
