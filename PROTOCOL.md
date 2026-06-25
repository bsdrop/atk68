# ATK68 protocol notes

Everything here was reverse-engineered from the ATK HUB web bundle and **verified
on real hardware** (firmware 1.25). It is enough to continue without re-doing the
reverse engineering.

## Device & transport

- ATK68, USB `1a81:207e` (Holtek). Self-reports as "Gtech ATK68".
- Three HID interfaces: keyboard, mouse, and the **vendor config interface**
  (usage page `0xFF01`, 64-byte in/out reports, **report id 0**). That last one
  is what we talk to (`/dev/hidraw*` on Linux). Picked by
  `usage_page >= 0xFF00` in `device::discover`.
- In the web app this is the `gtech-keyboard` controller `j7t` → `N7t` → `CHt`.
- **No feature reports** (GET_FEATURE times out), **no checksum**.

## Frame format

64-byte report, both directions:

```
byte 0 : commandId          (echoed back in the reply)
byte 1 : length             (number of data bytes)
byte 2 : resultCode (reply, 0 = ok) / selector or first data byte (request)
byte 3+: data
```

Send an **output report** `[0x00 report-id, commandId, length, …]`; the keyboard
answers with an **input report** whose byte 0 equals `commandId`. See
`Keyboard::exchange` in `src/device.rs`.

⚠️ **Request vs reply are shifted by one byte** for some commands: the reply
spends byte 2 on `resultCode`, so its data starts at byte 3, while a request can
reuse byte 2 for data/selector. Each command's `get`/`create` in the bundle
spells out the exact offsets — always check both.

## Command ids (`UTt` enum)

```
0x12 GetKeyboardInfo      0x13 GetKeyboardOnline    0x14 SetNKeyRollover
0x15 GetOneKeyAction      0x16 SetOneKeyAction      0x17 GetAllKeyAction
0x18 SetAllKeyAction      0x19 GetWasdSwitch        0x1a SetWasdSwitch
0x1b GetWinOrMac          0x1c SetWinOrMac          0x1d GetWinLock
0x1e SetWinLock           0x21 SaveStorage          0x22 ResetStorage
0x23 GetReportRate        0x24 SetReportRate        0x25 GetKeyDebounce
0x26 SetKeyDebounce       0x27 GetLowLatency        0x28 SetLowLatency
0x29 GetBottomTrigger     0x2a SetBottomTrigger     0x2b GetDeviceProfile
0x2c SetDeviceProfile     0x40 GetOneActuation      0x41 SetOneActuation
0x42 SetAllActuation      0x43 GetAllActuation      0x44 SetLightConfig
0x45 GetLightConfig       0x46 SetMacro             0x47 GetMacro
0x48 DataReported(async)  0x49 GetCustomLight       0x4a SetCustomLight
0x4b GetAllKeyActuation   0x5c SetAllKeyActuation   0x4c/4d ContinuousFastTrigger
0x4e GetDynamicKeyStroke  0x4f SetDynamicKeyStroke  0x50/51 ModTap
0x52/53 ToggleKey         0x54 GetOneFastTrigger    0x55 SetOneFastTrigger
0x56 GetAllFastTrigger    0x57 SetAllFastTrigger    0x58-5b ContFastTrigger
0x5d GetGlobalFastTrigger 0x5e SetGlobalFastTrigger 0x5f/60 RsKey
0x61/62 SocdKey           0x63/64 OksKey
```

## Verified encodings (implemented)

**Lighting** — `GetLightConfig 0x45` / `SetLightConfig 0x44` (bundle class is the
`Gjt`-based light command):
- GET request body **must use length 0**: `[0x00, 0x00]` (length 1 → `resultCode
  0xff` error). Reply data: `[enabled, mode, brightness, speed, R, G, B, colorful]`.
- SET body: `[len=8, enabled, mode, brightness, speed, R, G, B, 0, colorful]`.
- `brightness` 0–8, `speed` 0–4. Effect `mode` (back-light list `jRt`):
  `0` spectrum cycle, `1` static, `2` wave L→R, `3` breathing, `4` snake,
  `5` reactive (key-reaction), `6` rotate, `7` wave-to-centre, `8` fountain,
  `9` laser, `10` custom.

**Actuation + global rapid trigger** — `GetAllActuation 0x43` / `SetAllActuation
0x42` (class `C7t`):
- GET body `[0,0]`; reply data `[point, down, up]`.
- SET body `[len=3, point, down, up]`.
- Raw → mm uses the model's `travelStep`; ATK68 (PID 8318) step = 0.1 →
  `mm = raw * 0.1` (live default 10 = 1.0 mm). `down`/`up` = RT press/release.
- (Fine per-key trigger config elsewhere uses `mm = (raw + 1) / 100`.)

**Report rate** — `GetReportRate 0x23` / `SetReportRate 0x24` (class `f7t`):
- GET body `[0,0]`; reply data `[index]`. SET body `[len=1, index]`.
- index 0–6 = 125/250/500/1000/2000/4000/8000 Hz.
- ⚠️ A real change can re-enumerate the USB device (the `hidraw` handle breaks);
  apply it last and only if changed (see `config::Config::apply`).

**Per-key rapid trigger** — `GetOneFastTrigger 0x54` / `SetOneFastTrigger 0x55`
(class `z7t`): GET body `[2, row, col]` → reply data `[trigger]`; SET body
`[3, row, col, value]`. Matrix is row×col; non-existent positions return
`resultCode 0xff`. `GetAllFastTrigger 0x56` body `[1, row]` returns a whole row.

**Profile** — `GetDeviceProfile 0x2b` body `[0,0]` → `[index]`;
`SetDeviceProfile 0x2c` body `[1, index]`.

**Flash (NOT used)** — `SaveStorage 0x21` would commit RAM → flash and
`ResetStorage 0x22` factory-resets. This tool never sends them by design; they
are listed only for completeness.

## The bundle (for future RE)

Single minified JS (~15 MB, partly obfuscated) at
`https://bpcdn.atkgear.com/hub-v3/production/<version>/static/index-*.js`
(current `3.2.8`). Find the script path in `https://hub.atk.pro/` HTML. Command
classes extend `Gjt` (SIZE 64, baseOffset 3); their `static get()/create()`
methods define each command's body layout. The ATK68 controller class is `j7t`
("gtech-keyboard") → `N7t` → `CHt`.

## Deliberately excluded

- **Firmware / bootloader** (`bootloader_jump`, the `0xAA … 69 0F 47` boot-switch
  path): firmware images are encrypted + signed and a bad write bricks the
  device. Not implemented on purpose.
- **No network code** of any kind.
