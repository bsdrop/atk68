#!/usr/bin/env bash
# Build and install atk68 + the udev rule (Linux). One sudo for the rule + binary.
# After this you run `atk68` as your normal user with no further sudo.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> building release binary"
cargo build --release

echo "==> installing udev rule (needs sudo)"
sudo install -Dm644 packaging/60-atk68.rules /etc/udev/rules.d/60-atk68.rules
sudo udevadm control --reload
sudo udevadm trigger

echo "==> installing binary to /usr/local/bin"
sudo install -Dm755 target/release/atk68 /usr/local/bin/atk68

echo "Done. Replug the keyboard, then: atk68 status"
echo "Prefer no udev rule? Skip this script and run:"
echo "  sudo setcap cap_dac_override+ep target/release/atk68"
