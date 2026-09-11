# atk68

Small, offline, cross-platform configuration tool for **ATK / VXE magnetic-switch
keyboards** — a reviewable replacement for the official web app at hub.atk.pro,
built by reverse-engineering its HID protocol.

- **No cloud, no server, no browser.** Talks to the keyboard's HID interface
  directly. Zero network code.
- Settings normally apply to RAM. `storage save` can commit them to flash.
- **Linux / macOS / Windows**, single binary (Rust + hidapi).
- **i18n**: English, Korean, Japanese, Chinese (`--lang en|ko|ja|zh|auto`).

> ⚠️ **It might not work for you.** Only the **ATK68** (`1a81:207e`, firmware
> 1.25) is hardware-verified. Other models share the same controller and *should*
> work, but are untested. There is no warranty — use at your own risk. Settings
> apply to RAM and revert when you replug, so mistakes are recoverable.

## Build & install

```
cargo build --release          # -> target/release/atk68
```

On a clean Linux box you may need `libudev`/`systemd` dev headers and
`pkg-config`. macOS/Windows: just the Rust toolchain.

One-shot Linux install (binary + udev rule):

```
./packaging/install.sh
```

Cross builds (install the target first with `rustup target add`):

```
cargo build --release --target x86_64-pc-windows-gnu
cargo build --release --target aarch64-apple-darwin
```

## Usage

```
atk68 list                              # find attached keyboards
atk68 info                              # model + firmware version
atk68 status                            # snapshot of all decoded settings
atk68 light                             # show RGB lighting
atk68 light --color "#00ff00" --brightness 6
atk68 light --effect 5 --color "#ff0000"   # reactive (key-press) effect
atk68 light --off
atk68 actuation --point 1.2 --down 0.3 --up 0.3   # actuation + rapid trigger (mm)
atk68 rate --hz 8000                    # USB polling rate
atk68 tweak --set winlock=on --set rt=on   # toggles
atk68 profile 1                         # switch active profile
atk68 keyrt 0 1 5                       # per-key rapid trigger (row col value)
atk68 export ~/.atk68.conf              # save settings to a local file
atk68 apply  ~/.atk68.conf              # restore them in one shot
atk68 storage save                       # commit current RAM settings to flash
atk68 storage reset --yes                # factory-reset saved settings
atk68 raw "45 00 00"                    # debug: send cmd+body, dump reply
```

Normal changes apply to **RAM** and revert on replug; use `storage save` to
persist them. `--lang` works on every command, e.g. `atk68 status --lang ko`.

Profiles can also include toggles, for example `tweak.rt = on` and
`tweak.low-latency = on`. `export` writes all supported tweak values.

### Surviving flash wipes / glitches

Keep a local profile and re-apply it offline whenever the keyboard's saved
settings get wiped or an effect glitches:

```
atk68 export ~/.atk68.conf       # once, when settings are how you like them
atk68 apply  ~/.atk68.conf       # any time you need them back
```

It's a small editable text file. Put `atk68 apply ~/.atk68.conf` in your login
startup to re-apply automatically on boot.

## Linux permissions (no udev rule required)

**macOS and Windows need nothing.** On Linux, a normal user can only open the
keyboard's vendor `hidraw` node with *some* one-time privilege — this is the
kernel's security model and cannot be bypassed in code. You do **not** need a
udev rule, though; pick whichever you prefer (all one-time):

```
# A) No udev rule: grant the binary the capability to open the node.
sudo setcap cap_dac_override+ep ./target/release/atk68

# B) No rule, no setcap: just run it elevated.
sudo ./target/release/atk68 status

# C) A udev rule (then no sudo/setcap ever) — see packaging/60-atk68.rules:
sudo cp packaging/60-atk68.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger
```

If the device is found but can't be opened, the tool prints these options.

## Scope & models

Drives ATK/VXE magnetic-switch keyboards using the **gtech** controller (ATK68
family). It recognises a list of models for a friendly name and correct mm
scaling (`device::MODELS`); other keyboards on a known vendor id + vendor usage
page still work with defaults.

## Flash storage

`storage save` sends `SaveStorage (0x21)` and commits the current RAM settings.
Flash has limited write cycles, so use it only when needed. `storage reset --yes`
sends `ResetStorage (0x22)`. Firmware flashing remains excluded.

## Status

- ✅ Discovery + model table, info, status, **RGB** (incl. reactive),
  **global actuation + rapid trigger**, **per-key rapid trigger**, **polling
  rate**, **toggles** (winlock/wasd/mac/rt/…), **profile switch**,
  profile **export/apply**, i18n (en/ko/ja/zh).
- ⬜ Full key remap, per-key actuation, DKS / macros / SOCD — opcodes are mapped
  and reachable via `raw`; see `PROTOCOL.md` and `HACKING.md` to add them.

## Reverse-engineering / contributing

`PROTOCOL.md` — framing, opcode map, verified encodings.
`HACKING.md` — step-by-step guide to add the remaining features.

## License

MIT — see `LICENSE`. Not affiliated with or endorsed by ATK / VXE.
