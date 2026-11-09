#!/usr/bin/env bash
set -euo pipefail
if ((EUID != 0)); then
    echo "Run once as administrator: sudo $0" >&2
    exit 1
fi
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
install -Dm644 "$repo/packaging/linux/69-inkboard-wacom.rules" /etc/udev/rules.d/69-inkboard-wacom.rules
udevadm control --reload-rules
for event in /sys/class/input/event*; do
    if [[ -r "$event/device/id/vendor" ]] && [[ $(cat "$event/device/id/vendor") == 056a ]]; then
        udevadm trigger --action=change "$event"
    fi
done
udevadm settle
echo 'Wacom pen access configured for the active local user. Restart Inkboard (Wacom).'
