# Continuing atk68

How to add the remaining features without re-reverse-engineering. Read
`PROTOCOL.md` first — it has the framing, opcode map, and verified encodings.

## Layout

```
src/device.rs   transport + opcode map + Keyboard::exchange (low-level send/recv)
src/light.rs    one feature module: typed struct + get/set + encode/decode
src/actuation.rs same shape, for actuation/RT
src/config.rs   export/apply a local profile (offline flash-wipe recovery)
src/i18n.rs     en/ko/ja/zh strings + effect names
src/main.rs     clap CLI; one subcommand per feature
```

Each feature is a small self-contained module mirroring `light.rs`. Adding one
does not require understanding the others.

## Recipe to add a feature

1. **Get the bundle** (only for decoding; not needed at runtime):
   ```
   curl -s https://hub.atk.pro/ | grep -o 'static/index-[^"]*\.js'
   curl -s https://bpcdn.atkgear.com/hub-v3/production/3.2.8/static/index-XXXX.js -o /tmp/atk.js
   ```
2. **Find the command class.** Search the bundle for the `UTt` name, e.g.
   `SetAllKeyAction`, or the controller method (`setAllKeyAction`,
   `getAllKeysTravel`). The class extends `Gjt`; read its `static get()` /
   `static create()` to learn the body offsets and any selector byte
   (`baseOffset` = 3; remember the request/reply 1-byte shift in `PROTOCOL.md`).
3. **Add the opcode** to `device::op`.
4. **Probe it live first (safe):** `atk68 raw "<cmd> <body…>"` and read the reply.
   Confirm `resultCode` (byte 2) is `0`. This is how every encoding here was
   confirmed.
5. **Write the module** (copy `actuation.rs`): a typed struct, `get`, `set`
   (build the body exactly as the bundle's `create` does), plus any unit
   conversion. Use `Keyboard::exchange` and `Keyboard::data`.
6. **Wire the CLI** in `main.rs` (subcommand + handler) and add `i18n` strings.
7. **Verify by readback** — set a value, GET it back, compare bytes. Physically
   check if you can. Writes go to RAM and revert on replug, so this is safe.

### Worked example: how `actuation` was added

`getAllActuation` → `C7t.get()` (cmd `0x43`, no body) → reply data `[point,down,
up]`. `setAllActuation` → `C7t.create({point,down,up})` (cmd `0x42`, length 3,
sets bytes 2/3/4). That became `src/actuation.rs` (`get`/`set` + `mm`/`raw`
helpers) and `Cmd::Actuation` in `main.rs`. Verified: `actuation --point 1.5` →
`raw "43 00 00"` showed byte 3 = `0x0f`.

## Remaining features (opcodes ready)

Already done & verified: lighting, global actuation+RT, per-key RT (`z7t`),
report rate, toggles, profile switch. Remaining:

| Feature | Get / Set | Bundle class | Notes |
|---|---|---|---|
| Key remap | `0x17` / `0x18` | (all-key action) | needs HID keycode table + matrix (ATK68 = 2 layers, 5×15) |
| Per-key actuation | `0x40` / `0x41` | `w7t` | GET body `[3, a,b,c]`, SET `[6, a,b,c, point,down,up]`; confirm the 3-byte selector (RT uses 2 = row,col) |
| DKS / ModTap / Toggle | `0x4e/4f`, `0x50/51`, `0x52/53` | — | advanced per-key |
| Macro | `0x46` / `0x47` | — | sequence data |
| SOCD / RS / OKS | `0x61/62`, `0x5f/60`, `0x63/64` | — | resolve `setRs/setSocd/setOks` in `j7t` |
| Per-key RGB | `0x49` / `0x4a` | (custom light) | for effect `mode 10` custom |

Do **not** add flash writes (`SaveStorage 0x21`, `ResetStorage 0x22`) or firmware
commands — excluded by design.

## Safety rules

- Never send bootloader/firmware commands (see `PROTOCOL.md`). The tool has none.
- RAM writes are reversible (replug); `SaveStorage 0x21` persists to flash.
- A report-rate change can re-enumerate USB — apply last, only if changed.
- Always confirm a write with a GET readback.
