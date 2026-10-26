//! Command-line parsing and presentation.

use crate::i18n::{Lang, Text};
use crate::{actuation, config, device, light, tweaks};
use anyhow::{bail, Result};
use clap::{Args, Parser, Subcommand};
use device::{discover, Keyboard};
use hidapi::HidApi;

#[derive(Parser)]
#[command(
    name = "atk68",
    version,
    about = "Offline ATK68 keyboard config (no cloud, no browser)"
)]
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
    /// Commit RAM settings to the keyboard's flash or reset them.
    Storage {
        #[command(subcommand)]
        action: StorageCmd,
    },
    /// Debug: send raw hex (cmd + body) and dump the reply.
    Raw { hex: String },
}

#[derive(Subcommand)]
enum StorageCmd {
    /// Save current RAM settings to flash.
    Save,
    /// Factory-reset flash settings. Requires --yes.
    Reset {
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Args)]
struct LightArgs {
    /// Turn lighting on / off.
    #[arg(long, conflicts_with = "off")]
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
    /// Effect id (0=spectrum, 1=static, 3=breathing, …).
    #[arg(long)]
    effect: Option<u8>,
    /// Enable multicolour/rainbow.
    #[arg(long, conflicts_with = "no_rainbow")]
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

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let lang = Lang::parse(&cli.lang)?;
    match cli.cmd {
        Cmd::List => list(lang)?,
        Cmd::Info => {
            let kb = Keyboard::open()?;
            print_info(lang, &kb)?;
        }
        Cmd::Status => status(lang)?,
        Cmd::Light(a) => light_cmd(lang, a)?,
        Cmd::Actuation(a) => actuation_cmd(lang, a)?,
        Cmd::Rate { hz } => rate_cmd(lang, hz)?,
        Cmd::Tweak { set } => tweak_cmd(lang, &set)?,
        Cmd::Profile { index } => {
            let kb = Keyboard::open()?;
            if let Some(i) = index {
                kb.set_profile(i)?;
            }
            println!("{}: {}", lang.text(Text::Profile), kb.profile()?);
        }
        Cmd::Keyrt { row, col, value } => {
            let kb = Keyboard::open()?;
            if let Some(v) = value {
                kb.set_key_rt(row, col, v)?;
            }
            println!(
                "{} ({row},{col}): {}",
                lang.text(Text::KeyRt),
                kb.key_rt(row, col)?
            );
        }
        Cmd::Export { path } => {
            config::export(&Keyboard::open()?, &path)?;
            println!("{} {path}", lang.text(Text::ExportedProfile));
        }
        Cmd::Apply { path } => {
            config::apply_file(&path)?;
            println!("{} {path}", lang.text(Text::AppliedProfile));
        }
        Cmd::Storage { action } => {
            let kb = Keyboard::open()?;
            match action {
                StorageCmd::Save => {
                    kb.save_storage()?;
                    println!("{}", lang.text(Text::SavedFlash));
                }
                StorageCmd::Reset { yes: true } => {
                    kb.reset_storage()?;
                    println!("{}", lang.text(Text::ResetFlash));
                }
                StorageCmd::Reset { yes: false } => {
                    bail!("storage reset clears saved settings; repeat with --yes")
                }
            }
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

fn list(lang: Lang) -> Result<()> {
    let api = HidApi::new()?;
    let found = discover(&api);
    if found.is_empty() {
        println!("{}", lang.text(Text::NoDevice));
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
    print_info(lang, &kb)?;
    println!("{}: {}", lang.text(Text::Profile), kb.profile()?);
    println!(
        "{}: {}",
        lang.text(Text::ReportRate),
        rate_label(lang, kb.report_rate()?)
    );
    println!();
    print_actuation(lang, &actuation::get(&kb)?, kb.travel_step);
    println!();
    print_light(lang, &light::get(&kb)?);
    println!();
    println!("{}:", lang.text(Text::Toggles));
    print_tweaks(lang, &kb)?;
    Ok(())
}

fn print_info(lang: Lang, kb: &Keyboard) -> Result<()> {
    let info = kb.info()?;
    println!(
        "{}  {:04x}:{:04x}  {} {}",
        kb.name,
        info.vid,
        info.pid,
        lang.text(Text::Firmware),
        info.version
    );
    Ok(())
}

fn rate_label(lang: Lang, index: u8) -> String {
    device::REPORT_RATE_HZ
        .get(index as usize)
        .map(|hz| format!("{hz} Hz"))
        .unwrap_or_else(|| format!("{} {index}", lang.text(Text::Index)))
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
        l.brightness = light::validate_brightness(b)?;
    }
    if let Some(s) = a.speed {
        l.speed = light::validate_speed(s)?;
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
    println!("{}", lang.text(Text::AppliedRam));
    Ok(())
}

fn parse_tweak_updates(sets: &[String]) -> Result<Vec<(&'static tweaks::Toggle, u8)>> {
    sets.iter()
        .map(|pair| {
            let (key, value) = pair
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("use name=value"))?;
            let key = key.trim();
            let toggle =
                tweaks::find(key).ok_or_else(|| anyhow::anyhow!("unknown setting '{key}'"))?;
            Ok((toggle, tweaks::parse_value(value.trim())?))
        })
        .collect()
}

fn tweak_cmd(lang: Lang, sets: &[String]) -> Result<()> {
    let updates = parse_tweak_updates(sets)?;
    let kb = Keyboard::open()?;
    for (toggle, value) in updates {
        tweaks::set(&kb, toggle, value)?;
    }
    print_tweaks(lang, &kb)
}

fn print_tweaks(lang: Lang, kb: &Keyboard) -> Result<()> {
    for toggle in tweaks::TOGGLES {
        let value = tweaks::get(kb, toggle)?;
        let shown = match value {
            0 => lang.bool_value(false).to_string(),
            1 => lang.bool_value(true).to_string(),
            other => other.to_string(),
        };
        println!(
            "  {:<15} {:<4}  ({})",
            toggle.key,
            shown,
            lang.toggle_help(toggle.id)
        );
    }
    Ok(())
}

fn rate_cmd(lang: Lang, hz: Option<u32>) -> Result<()> {
    let kb = Keyboard::open()?;
    if let Some(target) = hz {
        kb.set_report_rate(device::nearest_rate(target))?;
    }
    println!(
        "{}: {}",
        lang.text(Text::ReportRate),
        rate_label(lang, kb.report_rate()?)
    );
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
    println!("{}", lang.text(Text::AppliedRam));
    Ok(())
}

fn print_actuation(lang: Lang, a: &actuation::Actuation, step: f32) {
    let line = |key: Text, raw: u8| {
        println!(
            "  {}: {:.2} mm ({})",
            lang.text(key),
            actuation::mm(raw, step),
            raw
        )
    };
    line(Text::Actuation, a.point);
    line(Text::RtPress, a.down);
    line(Text::RtRelease, a.up);
}

fn print_light(lang: Lang, l: &light::Light) {
    println!(
        "{}: {}",
        lang.text(Text::Lighting),
        lang.bool_value(l.enabled)
    );
    println!(
        "  {}: {} ({})",
        lang.text(Text::Effect),
        lang.effect_name(l.mode),
        l.mode
    );
    println!(
        "  {}: {}/{}",
        lang.text(Text::Brightness),
        l.brightness,
        light::BRIGHTNESS_MAX
    );
    println!(
        "  {}: {}/{}",
        lang.text(Text::Speed),
        l.speed,
        light::SPEED_MAX
    );
    println!(
        "  {}: #{:02X}{:02X}{:02X}",
        lang.text(Text::Color),
        l.rgb.0,
        l.rgb.1,
        l.rgb.2
    );
    println!(
        "  {}: {}",
        lang.text(Text::Rainbow),
        lang.bool_value(l.colorful)
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflicting_light_flags_are_rejected() {
        assert!(Cli::try_parse_from(["atk68", "light", "--on", "--off"]).is_err());
        assert!(Cli::try_parse_from(["atk68", "light", "--rainbow", "--no-rainbow"]).is_err());
    }

    #[test]
    fn tweak_updates_are_validated_before_use() {
        let updates = parse_tweak_updates(&["rt=on".into(), "low-latency=1".into()]).unwrap();
        assert_eq!(updates.len(), 2);
        assert!(parse_tweak_updates(&["rt=on".into(), "unknown=off".into()]).is_err());
    }

    #[test]
    fn unknown_rate_is_shown_as_index() {
        assert_eq!(rate_label(Lang::En, 99), "index 99");
    }
}
