//! HID transport + command layer for the ATK68 (gtech controller `j7t`).
//!
//! Verified on real hardware: the keyboard speaks on its vendor interface
//! (usage page 0xFF01) using **output reports with report-id 0** and replies
//! with input reports. No feature reports, no checksum, and — crucially — no
//! browser/WebHID and no udev rule are required to reach it.
//!
//! Frame (64 bytes): `[commandId, length, resultCode/selector, data…]`.
//! A reply echoes `commandId` in byte 0, carries `resultCode` in byte 2, and
//! its data starts at byte 3. Command ids are the firmware's `UTt` enum.

use anyhow::{anyhow, bail, Context, Result};
use hidapi::{HidApi, HidDevice};

pub const KNOWN_VIDS: &[u16] = &[0x1A81, 0x3554, 0x373B];
const VENDOR_USAGE_MIN: u16 = 0xFF00;
pub const REPORT_LEN: usize = 64;

/// Known magnetic-switch keyboards that use the same `gtech` controller as the
/// ATK68. `step` is the actuation/travel resolution in mm. Only the ATK68 is
/// hardware-verified; the rest share the controller and should work, but are
/// listed mainly for a friendly name + correct mm scaling. Unlisted devices
/// that match a known VID + vendor usage page still work with defaults.
pub struct Model {
    pub vid: u16,
    pub pid: u16,
    pub name: &'static str,
    pub step: f32,
}

pub const MODELS: &[Model] = &[
    Model { vid: 0x1A81, pid: 0x207E, name: "ATK68", step: 0.1 }, // verified
    Model { vid: 0x1A81, pid: 0x2075, name: "ATK75 G", step: 0.1 },
    Model { vid: 0x1A81, pid: 0x2082, name: "ATK75 L", step: 0.1 },
    Model { vid: 0x1A81, pid: 0x2083, name: "ATK68 L", step: 0.02 },
    Model { vid: 0x373B, pid: 0x1005, name: "ATK68 Air", step: 0.02 },
    Model { vid: 0x373B, pid: 0x2002, name: "ATK68 V2 Pro", step: 0.1 },
    Model { vid: 0x373B, pid: 0x2029, name: "ATK68 V2", step: 0.1 },
    Model { vid: 0x373B, pid: 0x1259, name: "ATK68 V4", step: 0.1 },
    Model { vid: 0x373B, pid: 0x21B8, name: "ATK68 V4", step: 0.1 },
    Model { vid: 0x373B, pid: 0x21C6, name: "ATK68 RX", step: 0.1 },
    Model { vid: 0x373B, pid: 0x2169, name: "ATK EDGE75", step: 0.1 },
    Model { vid: 0x373B, pid: 0x21BC, name: "EDGE 63HE Ultimate", step: 0.1 },
    Model { vid: 0x373B, pid: 0x2188, name: "ATK75 V2", step: 0.1 },
    Model { vid: 0x373B, pid: 0x21C9, name: "ATK RS7 Turbo", step: 0.1 },
    Model { vid: 0x373B, pid: 0x1177, name: "ATK x QK Hex80", step: 0.1 },
];

const DEFAULT_STEP: f32 = 0.1;

pub fn model_of(vid: u16, pid: u16) -> Option<&'static Model> {
    MODELS.iter().find(|m| m.vid == vid && m.pid == pid)
}

/// Firmware command ids (subset of `UTt`, names as in the web bundle).
/// Some are listed for the protocol map before their feature is implemented.
#[allow(dead_code)]
pub mod op {
    pub const GET_KEYBOARD_INFO: u8 = 0x12;
    // 0x21 SaveStorage / 0x22 ResetStorage write flash — intentionally NOT
    // exposed (the tool is RAM-only; see README). Listed for the protocol map.
    pub const GET_REPORT_RATE: u8 = 0x23;
    pub const SET_REPORT_RATE: u8 = 0x24;
    pub const GET_DEVICE_PROFILE: u8 = 0x2B;
    pub const SET_DEVICE_PROFILE: u8 = 0x2C;
    pub const GET_ONE_FAST_TRIGGER: u8 = 0x54;
    pub const SET_ONE_FAST_TRIGGER: u8 = 0x55;
    pub const SET_ALL_ACTUATION: u8 = 0x42;
    pub const SET_LIGHT: u8 = 0x44;
    pub const GET_LIGHT: u8 = 0x45;
    pub const GET_ALL_ACTUATION: u8 = 0x43;
    pub const DATA_REPORTED: u8 = 0x48; // async, unsolicited
}

#[derive(Clone)]
pub struct Found {
    pub vid: u16,
    pub pid: u16,
    pub product: String,
    pub path: String,
    pub usage_page: u16,
}

/// Enumerate attached ATK/VXE vendor config interfaces.
pub fn discover(api: &HidApi) -> Vec<Found> {
    api.device_list()
        .filter(|d| KNOWN_VIDS.contains(&d.vendor_id()) && d.usage_page() >= VENDOR_USAGE_MIN)
        .map(|d| Found {
            vid: d.vendor_id(),
            pid: d.product_id(),
            product: d.product_string().unwrap_or("?").to_string(),
            path: d.path().to_string_lossy().into_owned(),
            usage_page: d.usage_page(),
        })
        .collect()
}

pub struct Keyboard {
    dev: HidDevice,
    /// Friendly model name (falls back to the USB product string).
    pub name: String,
    /// Actuation/travel resolution in mm for this model.
    pub travel_step: f32,
}

impl Keyboard {
    pub fn open() -> Result<Keyboard> {
        let api = HidApi::new().context("init hidapi")?;
        let target = discover(&api)
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("no ATK/VXE keyboard found (is it plugged in?)"))?;
        let model = model_of(target.vid, target.pid);
        let name = model.map(|m| m.name.to_string()).unwrap_or(target.product.clone());
        let travel_step = model.map(|m| m.step).unwrap_or(DEFAULT_STEP);
        let cpath = std::ffi::CString::new(target.path.clone())?;
        // The device was found (enumeration reads sysfs), so an open failure here
        // is almost always a permission problem on the hidraw node.
        api.open_path(&cpath)
            .map(|dev| Keyboard { dev, name, travel_step })
            .map_err(|e| anyhow!(permission_hint(&target.path, e)))
    }

    /// Send `[cmd, body…]` as a report-id-0 output report and return the
    /// matching reply (input report whose byte 0 equals `cmd`). `body` is the
    /// bytes from index 1 on, i.e. `[length, selector/data…]`.
    pub fn exchange(&self, cmd: u8, body: &[u8]) -> Result<[u8; REPORT_LEN]> {
        if body.len() + 2 > REPORT_LEN {
            bail!("command body too long");
        }
        let mut out = vec![0u8; REPORT_LEN + 1]; // out[0] = report id 0
        out[1] = cmd;
        out[2..2 + body.len()].copy_from_slice(body);
        self.dev.write(&out).context("hid write")?;

        // Read replies, skipping unsolicited async reports, until ours arrives.
        for _ in 0..16 {
            let mut reply = [0u8; REPORT_LEN];
            let n = self.dev.read_timeout(&mut reply, 1000).context("hid read")?;
            if n == 0 {
                break;
            }
            if reply[0] == cmd {
                return Ok(reply);
            }
            if reply[0] != op::DATA_REPORTED && reply[0] != 0 {
                // A reply for a different command id — not expected here.
            }
        }
        bail!("no reply for command 0x{cmd:02x}")
    }

    /// The data region of a reply (`length` bytes starting after the header).
    pub fn data(reply: &[u8; REPORT_LEN]) -> &[u8] {
        let len = (reply[1] as usize).min(REPORT_LEN - 3);
        &reply[3..3 + len]
    }

    pub fn info(&self) -> Result<Info> {
        let r = self.exchange(op::GET_KEYBOARD_INFO, &[0, 0])?;
        let d = Self::data(&r);
        Ok(Info {
            vid: u16::from_be_bytes([d[0], d[1]]),
            pid: u16::from_be_bytes([d[2], d[3]]),
            version: format!("{}.{:02x}", d[6], d[7]),
        })
    }

    /// First data byte of a simple GET, or 0 on empty/error. Used for the
    /// single-value reads (active profile, report-rate index).
    fn get_byte(&self, cmd: u8) -> Result<u8> {
        let r = self.exchange(cmd, &[0, 0])?;
        Ok(Self::data(&r).first().copied().unwrap_or(0))
    }

    pub fn profile(&self) -> Result<u8> {
        self.get_byte(op::GET_DEVICE_PROFILE)
    }

    pub fn set_profile(&self, index: u8) -> Result<()> {
        self.exchange(op::SET_DEVICE_PROFILE, &[1, index])?;
        Ok(())
    }

    /// Per-key rapid-trigger value (keyed by matrix row/col). Errors for matrix
    /// positions that don't exist on this layout.
    pub fn key_rt(&self, row: u8, col: u8) -> Result<u8> {
        let r = self.exchange(op::GET_ONE_FAST_TRIGGER, &[2, row, col])?;
        if r[2] != 0 {
            bail!("no key at row {row}, col {col}");
        }
        Ok(Self::data(&r).first().copied().unwrap_or(0))
    }

    pub fn set_key_rt(&self, row: u8, col: u8, value: u8) -> Result<()> {
        let r = self.exchange(op::SET_ONE_FAST_TRIGGER, &[3, row, col, value])?;
        if r[2] != 0 {
            bail!("could not set rapid trigger at row {row}, col {col}");
        }
        Ok(())
    }

    /// Report-rate index (see `REPORT_RATE_HZ`).
    pub fn report_rate(&self) -> Result<u8> {
        self.get_byte(op::GET_REPORT_RATE)
    }

    pub fn set_report_rate(&self, index: u8) -> Result<()> {
        self.exchange(op::SET_REPORT_RATE, &[1, index])?;
        Ok(())
    }

    /// Debug exchange: raw bytes after the report id, used by `atk68 raw`.
    pub fn raw(&self, payload: &[u8]) -> Result<[u8; REPORT_LEN]> {
        if payload.is_empty() {
            bail!("empty payload");
        }
        self.exchange(payload[0], &payload[1..])
    }
}

pub struct Info {
    pub vid: u16,
    pub pid: u16,
    pub version: String,
}

/// Report-rate index → polling rate in Hz (firmware `sHt` order).
pub const REPORT_RATE_HZ: &[u32] = &[125, 250, 500, 1000, 2000, 4000, 8000];

/// Build a helpful message when the device is present but cannot be opened.
/// On Linux a normal user can only open a vendor `hidraw` node with one-time
/// privilege (the kernel's security model — it cannot be bypassed in code);
/// macOS/Windows need nothing, so there the raw error is shown.
fn permission_hint(path: &str, err: hidapi::HidError) -> String {
    if !cfg!(target_os = "linux") {
        return format!("cannot open {path}: {err}");
    }
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "atk68".into());
    format!(
        "cannot open {path}: {err}\n\
         The keyboard node needs access. Pick ONE of these (each is one-time):\n  \
         • no udev rule — grant the binary capability:\n      \
         sudo setcap cap_dac_override+ep {exe}\n  \
         • or just run it elevated:\n      sudo {exe} …\n  \
         • or keep a udev rule (see README.md → Linux permissions)"
    )
}
