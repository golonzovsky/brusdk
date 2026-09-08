# Capture tools

All observation happens here. None of these files read the Brady SDK; `drive.mjs`
loads it as a black box from the brady-m511 skill's `node_modules` and calls the
same public methods `server/printer.mjs` in the label studio calls.

- `hook.mjs`: preload with `node --import`. When `BRADY_CAPTURE=<dir>` is set it wraps the
  `webbluetooth` polyfill's characteristic/server prototypes and appends one JSON line per
  write, read, notification, connect, discovery step to `<dir>/<timestamp>-<label>.jsonl`.
  `BRADY_CAPTURE_LABEL` names the file. Fields: `t_ms` since process start, `ev`
  (`tx`, `tx-done`, `rx`, `rx-read`, `connect`, `services`, `characteristics`, ...),
  `char` (first 8 hex digits of the characteristic UUID), `mode` (`req` = write with
  response, `cmd` = write without response), `hex`.
- `drive.mjs`: `info | idle --seconds N | print --image p.png [--copies] [--cut 0|1|2] | feed | cut`.
- `cap.sh <session-dir> <label> <drive args>`: runs drive.mjs under the hook and tees the log.
- `gatt-dump.mjs`: polyfill only, no SDK. Enumerates services/characteristics/descriptors,
  reads readable values, subscribes to everything and records unsolicited traffic.

Only one BLE client at a time: stop or leave disconnected the label studio sidecar on :5178
before running any of these, and press the printer's power button first.
