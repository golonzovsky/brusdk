# Status

_Updated: 2026-09-08 03:45 CEST_

## Known (from the device or the wire)

- Advertisement, GATT layout, request framing (constant 16-byte token + u32 LE length
  + JSON), request fragmentation, indication reassembly: PROTOCOL.md 1-2.
- Handshake = one `PropertySubscribeRequests` for 24 IDs; printer answers with
  `PropertyGetResponses` twice. Decoded IDs: 000C width mils, 0016 remaining %,
  004D part number, 0020 firmware, 0001 battery class, 0029 last job status text,
  0009 last job failed: PROTOCOL.md 3.
- Print job command stream and raster encoding fully reproduced: `src/job.rs`
  matches all 17 captured jobs (sessions 1 and 2: probes, copies, cut options,
  offsets, odd width) byte for byte, fragmentation included (`cargo test`).
  Blank jobs are rejected by the printer.
- Copies = `C` field + repeated label block; cut option = `M` byte; SDK x/y
  offsets move the raster (default placement 36 rows down); feed = set 0007,
  cut = set 0004: PROTOCOL.md 4.5, 5.
- The Rust crate connects, handshakes, reads properties and prints on its own:
  `brusdk print --send` of the band probe was accepted (`<job id>:Successful`,
  2026-09-08 03:10, captures/2026-09-08-s3); Alex confirmed it matches the SDK's
  print. Feed and cut from Rust ran without error (03:35).
- HCI cross-check done: MTU 156, every hook write byte-identical on the expected
  ATT handle, indication streams contiguous (PROTOCOL.md 1).
- `brusdk serve` offers the sidecar's HTTP API (GET /api/printer, POST
  /api/connect|disconnect|print|dryrun|feed|cut) with the same JSON shapes;
  smoke-tested offline. xOffsetIn keeps the SDK contract (default -0.12 in
  cancels the 36-row default placement).
- The supply name, dpi and die-cut flag are not on the wire; the SDK derives them
  from the part number (004D).

## Guessed

- Job fields D (+0001), c (00), p/o/O/b (+00), A/Q/a, G, K/000b and the K/0009
  supply-name prefix are constant in every capture; their meaning is unknown and
  they are replayed as-is. Battery "High" to percent mapping unknown. Whether the
  printer answers a PropertySetRequest (feed/cut) is unknown (driver disconnected
  after 3 s).
- Whether the printer accepts a different session token, a bigger fragment, or a
  literal column where the SDK would send run lengths: untested.

## Blocked / open

- Physical comparison of the Rust band print with the SDK's: pending Alex.
- Nothing blocking. The label studio printed through `brusdk serve` on :5178
  in place of the Node sidecar (2026-09-08 03:45, Alex: "worked well").
  Integration into crates/label-server (spawn/embed brusdk instead of node)
  is the label studio's next step.
- Six cartridges in `src/supply.rs` (5153508 WT-BK, 5072987 OR-BK, 5072986 CL-BK,
  5072905 CL-WT, 5072903 BK-WT, 5073028 M4C-250-7641-YL 0.355 in sleeve). Flags 0006/0066 = latch open;
  the other ten flags and non-"High" battery classes are unmapped.
- Head extent on the 450-row canvas (rows 432-449) untested; one full-height
  column print would settle it.
- Narrow supplies: the canvas height rule (0.355 in -> 106 or 107 rows) and the
  SDK's row placement on them are unverified; one dot print on the M4C sleeve
  under capture would settle both. Until then `brusdk serve` rounds
  (107 rows) and places at the SDK's 36-row default minus the studio's offset.
- The polyfill's scan is flaky: about one connect in four times out at 30 s,
  60 s works; the Rust scanner needs the same patience.

## Done

- Repo scaffold, LEGAL.md, capture hook (`tools/capture/hook.mjs`), SDK black-box
  driver (`tools/capture/drive.mjs`, `cap.sh`), GATT dump script
  (`tools/capture/gatt-dump.mjs`), Rust crate with `scan`/`gatt`/`probes`.
- Capture plan (sessions 0, 1, 2) sent to Alex 2026-09-08; self-test of the
  hook with the printer asleep passes.
