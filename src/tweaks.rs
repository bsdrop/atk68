//! Single-byte keyboard settings using shared GET/SET framing.

use crate::device::{first_byte, Keyboard};
use anyhow::Result;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToggleId {
    Winlock,
    Wasd,
    Mac,
    Rt,
    LowLatency,
    BottomTrigger,
    Debounce,
}

pub struct Toggle {
    pub id: ToggleId,
    pub key: &'static str,
    pub get: u8,
    pub set: u8,
}

pub const TOGGLES: &[Toggle] = &[
    Toggle {
        id: ToggleId::Winlock,
        key: "winlock",
        get: 0x1D,
        set: 0x1E,
    },
    Toggle {
        id: ToggleId::Wasd,
        key: "wasd",
        get: 0x19,
        set: 0x1A,
    },
    Toggle {
        id: ToggleId::Mac,
        key: "mac",
        get: 0x1B,
        set: 0x1C,
    },
    Toggle {
        id: ToggleId::Rt,
        key: "rt",
        get: 0x5D,
        set: 0x5E,
    },
    Toggle {
        id: ToggleId::LowLatency,
        key: "low-latency",
        get: 0x27,
        set: 0x28,
    },
    Toggle {
        id: ToggleId::BottomTrigger,
        key: "bottom-trigger",
        get: 0x29,
        set: 0x2A,
    },
    Toggle {
        id: ToggleId::Debounce,
        key: "debounce",
        get: 0x25,
        set: 0x26,
    },
];

pub fn find(key: &str) -> Option<&'static Toggle> {
    TOGGLES.iter().find(|t| t.key == key)
}

pub fn get(kb: &Keyboard, t: &Toggle) -> Result<u8> {
    let r = kb.exchange(t.get, &[0, 0])?;
    first_byte(t.get, &r)
}

pub fn set(kb: &Keyboard, t: &Toggle, value: u8) -> Result<()> {
    kb.exchange(t.set, &[1, value])?;
    Ok(())
}

/// Parse a value like `on`/`off`/`1`/`0`/`3`.
pub fn parse_value(v: &str) -> Result<u8> {
    match v.to_ascii_lowercase().as_str() {
        "on" | "true" | "yes" => Ok(1),
        "off" | "false" | "no" => Ok(0),
        n => n
            .parse()
            .map_err(|_| anyhow::anyhow!("bad value '{v}' (on/off/number)")),
    }
}
