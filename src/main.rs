//! atk68 — offline, cross-platform configuration tool for the ATK68 keyboard.
//!
//! Talks to the keyboard's vendor HID interface directly (no cloud, no browser).
//! Settings apply to RAM immediately and revert on replug; this tool never
//! writes the keyboard's flash. Use `export`/`apply` to keep them locally.

mod actuation;
mod config;
mod device;
mod i18n;
mod light;
mod tweaks;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use device::{discover, Keyboard};
use hidapi::HidApi;
use i18n::{effect_name, tr, Lang};

#[derive(Parser)]
#[command(name = "atk68", version, about = "Offline ATK68 keyboard config (no cloud, no browser)")]
struct Cli {
    /// UI language: en | ko | ja | zh | auto.
    #[arg(long, global = true, default_value = "auto")]
    lang: String,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List attached ATK/VXE keyboards.
    List,
    /// Show keyboard model and firmware info.
    Info,
    /// Read-only snapshot of all currently-decoded settings.
    Status,
    /// Show or change RGB lighting. With no options, prints the current state.
    Light(LightArgs),
    /// Show or change global actuation + rapid trigger (millimetres).
    Actuation(ActuationArgs),
    /// Show or set the USB polling rate in Hz.
    Rate {
        /// Target Hz (125/250/500/1000/2000/4000/8000); nearest is used.
        #[arg(long)]
        hz: Option<u32>,
    },
    /// Show or change simple toggles (winlock, wasd, mac, rt, …).
    Tweak {
        /// Repeatable name=value, e.g. --set winlock=on --set rt=on.
        #[arg(long = "set", value_name = "NAME=VALUE")]
        set: Vec<String>,
    },
    /// Show the active profile, or switch to index N (0-based).
    Profile { index: Option<u8> },
    /// Read or set a single key's rapid-trigger value by matrix row/col.
    Keyrt {
        row: u8,
        col: u8,
        /// New value; omit to just read.
        value: Option<u8>,
    },
    /// Export the keyboard's current settings to a local profile file.
    Export { path: String },
    /// Apply a local profile file to the keyboard (RAM).
    Apply { path: String },
    /// Debug: send raw hex (cmd + body) and dump the reply.
    Raw { hex: String },
}

#[derive(Args)]
struct LightArgs {
    /// Turn lighting on / off.
    #[arg(long)]
    on: bool,
    #[arg(long)]
    off: bool,
    /// Color as #RRGGBB or r,g,b.
    #[arg(long)]
    color: Option<String>,
    /// Brightness 0..=8.
    #[arg(long)]
    brightness: Option<u8>,
    /// Animation speed 0..=4.
    #[arg(long)]
    speed: Option<u8>,
    /// Effect id (0=off,1=static,2=breathing,…).
    #[arg(long)]
    effect: Option<u8>,
    /// Enable multicolour/rainbow.
    #[arg(long)]
    rainbow: bool,
    /// Disable multicolour/rainbow.
    #[arg(long)]
    no_rainbow: bool,
}

#[derive(Args)]
struct ActuationArgs {
    /// Actuation depth in mm (0.1–4.0).
    #[arg(long)]
    point: Option<f32>,
    /// Rapid-trigger press sensitivity in mm.
    #[arg(long)]
    down: Option<f32>,
    /// Rapid-trigger release sensitivity in mm.
    #[arg(long)]
    up: Option<f32>,
}

fn main() -> Result<()> {
    // Behave like a normal Unix tool when output is piped to e.g. `head`:
    // exit quietly on a closed pipe instead of panicking.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    let cli = Cli::parse();
    let lang = Lang::parse(&cli.lang);
    match cli.cmd {
        Cmd::List => list()?,
        Cmd::Info => {
            let kb = Keyboard::open()?;
            let info = kb.info()?;
            println!(
                "{}  {:04x}:{:04x}  firmware {}",
                kb.name, info.vid, info.pid, info.version
            );
        }
        Cmd::Status => status(lang)?,
        Cmd::Light(a) => light_cmd(lang, a)?,
        Cmd::Actuation(a) => actuation_cmd(lang, a)?,
        Cmd::Rate { hz } => rate_cmd(hz)?,
        Cmd::Tweak { set } => tweaks::run(&Keyboard::open()?, &set)?,
        Cmd::Profile { index } => {
            let kb = Keyboard::open()?;
            if let Some(i) = index {
                kb.set_profile(i)?;
            }
            println!("profile: {}", kb.profile()?);
        }
        Cmd::Keyrt { row, col, value } => {
            let kb = Keyboard::open()?;
            if let Some(v) = value {
                kb.set_key_rt(row, col, v)?;
            }
            println!("key ({row},{col}) rapid trigger: {}", kb.key_rt(row, col)?);
        }
        Cmd::Export { path } => {
            config::export(&Keyboard::open()?, &path)?;
            println!("exported profile to {path}");
        }
        Cmd::Apply { path } => {
            config::apply_file(&Keyboard::open()?, &path)?;
            println!("applied {path}");
        }
        Cmd::Raw { hex } => {
            let payload: Vec<u8> = hex
                .split_whitespace()
                .map(|h| u8::from_str_radix(h, 16))
                .collect::<Result<_, _>>()?;
            let reply = Keyboard::open()?.raw(&payload)?;
            print!("reply:");
            for (i, b) in reply.iter().enumerate() {
                if i % 16 == 0 {
                    print!("\n  ");
                }
                print!("{:02x} ", b);
            }
            println!();
        }
    }
    Ok(())
}

fn list() -> Result<()> {
    let api = HidApi::new()?;
    let found = discover(&api);
    if found.is_empty() {
        println!("No ATK/VXE keyboard found.");
    }
    for d in found {
        println!(
            "{:04x}:{:04x}  usage_page=0x{:04x}  \"{}\"\n  {}",
            d.vid, d.pid, d.usage_page, d.product, d.path
        );
    }
    Ok(())
}

fn status(lang: Lang) -> Result<()> {
    let kb = Keyboard::open()?;
    let info = kb.info()?;
    println!("{}  {:04x}:{:04x}  firmware {}", kb.name, info.vid, info.pid, info.version);
    println!("profile: {}", kb.profile()?);
    let rate = kb.report_rate()? as usize;
    let hz = device::REPORT_RATE_HZ
        .get(rate)
        .map(|h| format!("{h} Hz"))
        .unwrap_or_else(|| format!("index {rate}"));
    println!("report rate: {hz}");
    println!();
    print_actuation(lang, &actuation::get(&kb)?, kb.travel_step);
    println!();
    print_light(lang, &light::get(&kb)?);
    Ok(())
}

fn light_cmd(lang: Lang, a: LightArgs) -> Result<()> {
    let kb = Keyboard::open()?;
    let mut l = light::get(&kb)?;

    let changing = a.on
        || a.off
        || a.color.is_some()
        || a.brightness.is_some()
        || a.speed.is_some()
        || a.effect.is_some()
        || a.rainbow
        || a.no_rainbow;

    if !changing {
        print_light(lang, &l);
        return Ok(());
    }

    if a.on {
        l.enabled = true;
    }
    if a.off {
        l.enabled = false;
    }
    if let Some(c) = a.color {
        l.rgb = light::parse_rgb(&c)?;
    }
    if let Some(b) = a.brightness {
        l.brightness = b.min(light::BRIGHTNESS_MAX);
    }
    if let Some(s) = a.speed {
        l.speed = s.min(light::SPEED_MAX);
    }
    if let Some(e) = a.effect {
        l.mode = e;
    }
    if a.rainbow {
        l.colorful = true;
    }
    if a.no_rainbow {
        l.colorful = false;
    }

    light::set(&kb, &l)?;
    print_light(lang, &l);
    println!("{}", tr(lang, "not_saved"));
    Ok(())
}

fn rate_cmd(hz: Option<u32>) -> Result<()> {
    let kb = Keyboard::open()?;
    if let Some(target) = hz {
        // Pick the index whose Hz is closest to the requested value.
        let index = device::REPORT_RATE_HZ
            .iter()
            .enumerate()
            .min_by_key(|(_, h)| h.abs_diff(target))
            .map(|(i, _)| i as u8)
            .unwrap_or(0);
        kb.set_report_rate(index)?;
    }
    let idx = kb.report_rate()? as usize;
    let hz = device::REPORT_RATE_HZ.get(idx).copied().unwrap_or(0);
    println!("report rate: {hz} Hz");
    Ok(())
}

fn actuation_cmd(lang: Lang, a: ActuationArgs) -> Result<()> {
    let kb = Keyboard::open()?;
    let mut act = actuation::get(&kb)?;

    if a.point.is_none() && a.down.is_none() && a.up.is_none() {
        print_actuation(lang, &act, kb.travel_step);
        return Ok(());
    }
    if let Some(p) = a.point {
        act.point = actuation::raw(p, kb.travel_step);
    }
    if let Some(d) = a.down {
        act.down = actuation::raw(d, kb.travel_step);
    }
    if let Some(u) = a.up {
        act.up = actuation::raw(u, kb.travel_step);
    }
    actuation::set(&kb, &act)?;
    print_actuation(lang, &act, kb.travel_step);
    println!("{}", tr(lang, "not_saved"));
    Ok(())
}

fn print_actuation(lang: Lang, a: &actuation::Actuation, step: f32) {
    let line = |k: &'static str, raw: u8| {
        println!("  {}: {:.2} mm ({})", tr(lang, k), actuation::mm(raw, step), raw)
    };
    line("actuation", a.point);
    line("rt_press", a.down);
    line("rt_release", a.up);
}

fn print_light(lang: Lang, l: &light::Light) {
    let on = if l.enabled { "on" } else { "off" };
    println!("{}: {}", tr(lang, "lighting"), on);
    println!("  {}: {} ({})", tr(lang, "effect"), effect_name(lang, l.mode), l.mode);
    println!("  {}: {}/{}", tr(lang, "brightness"), l.brightness, light::BRIGHTNESS_MAX);
    println!("  {}: {}/{}", tr(lang, "speed"), l.speed, light::SPEED_MAX);
    println!(
        "  {}: #{:02X}{:02X}{:02X}",
        tr(lang, "color"),
        l.rgb.0,
        l.rgb.1,
        l.rgb.2
    );
    println!("  {}: {}", tr(lang, "rainbow"), l.colorful);
}
