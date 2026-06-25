//! Global actuation + rapid-trigger settings (gtech `C7t`, cmd 0x42/0x43).
//!
//! GET 0x43 reply data: `[point, down, up]`; SET 0x42 body: `[len=3, point, down, up]`.
//! Values are raw firmware units; the ATK68 (PID 0x207E) uses a 0.1 mm step, so
//! `mm = raw * 0.1` (a live default of 10 = 1.0 mm). `down`/`up` are the rapid-
//! trigger press / release sensitivities.

use crate::device::{op, Keyboard};
use anyhow::{bail, Result};

const TRAVEL_MAX_MM: f32 = 4.0;

#[derive(Clone, Copy)]
pub struct Actuation {
    pub point: u8,
    pub down: u8,
    pub up: u8,
}

pub fn mm(raw: u8, step: f32) -> f32 {
    raw as f32 * step
}

pub fn raw(mm: f32, step: f32) -> u8 {
    let max = (TRAVEL_MAX_MM / step).round().min(255.0);
    (mm / step).round().clamp(1.0, max) as u8
}

pub fn get(kb: &Keyboard) -> Result<Actuation> {
    let r = kb.exchange(op::GET_ALL_ACTUATION, &[0, 0])?;
    if r[2] != 0 {
        bail!("keyboard returned error 0x{:02x} reading actuation", r[2]);
    }
    let d = Keyboard::data(&r);
    if d.len() < 3 {
        bail!("short actuation reply ({} bytes)", d.len());
    }
    Ok(Actuation {
        point: d[0],
        down: d[1],
        up: d[2],
    })
}

pub fn set(kb: &Keyboard, a: &Actuation) -> Result<()> {
    kb.exchange(op::SET_ALL_ACTUATION, &[3, a.point, a.down, a.up])?;
    Ok(())
}
