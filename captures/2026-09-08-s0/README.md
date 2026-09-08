# Session 0, 2026-09-08 (no printing)

Cartridge: M5C-1500-595-WT-BK-BULK (part 5153508), 90 % remaining, battery 100 %.
Printer woken by power button before each connection. Label-studio sidecar on :5178
left running but disconnected.

| File | What |
|---|---|
| `gatt-node.json` | GATT enumeration with the webbluetooth polyfill only (no SDK), 15 s listen after subscribing |
| `gatt-rust.txt` | Same with btleplug (`brusdk gatt`), includes advertisement data |
| `*-idle60.jsonl`, `idle60.log` | SDK connect, handshake, 60 s idle, SDK disconnect |
| `*-sleepwatch.jsonl`, `sleepwatch.log` | SDK connect then stay connected until the printer drops the link |

Decode with `python3 tools/capture/decode.py <jsonl>`.

`hci.pklg` (PacketLogger, 4 MB) is kept locally only, not committed: the system-wide logging
profile captured every Bluetooth device from 2026-09-03 on. `tools/capture/hci-compare.py`
documents how it was checked against the hook captures.
