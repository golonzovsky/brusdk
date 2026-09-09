# brusdk

A Rust driver for the Brady M511 label printer over Bluetooth LE, written without
Brady's SDK. It discovers the printer, reads its state, and prints 1-bit rasters
with copies and cut options, and it serves the same HTTP API as the Node sidecar the
brady-label-studio (a private label editor) used to run, so the studio can print
through it unchanged.

## Why

Brady ships no macOS software for the M511 and its Web SDK is a licensed black box
(`@bradycorporation/brady-web-sdk`), which the label studio had to run headless in
Node under a Web Bluetooth polyfill. This crate replaces that sidecar with a native
implementation whose behaviour is fully documented.

## How it was made

Nothing in the SDK was read. The protocol was derived from what crosses the
Bluetooth link: an opt-in hook at the polyfill boundary recorded every write and
indication while the unmodified SDK printed probe images with known pixels (single
dots, full rows and columns, checkers, odd widths, copies, cut options, offsets),
plus a raw HCI trace from Apple's PacketLogger to confirm the hook saw everything.
`PROTOCOL.md` states each fact with the capture file and byte offsets behind it, and
`LEGAL.md` describes the method. The encoder is checked byte for byte against every
captured SDK job (`cargo test`).

## What the printer speaks

- One GATT service `FD1C` with a job characteristic, a request characteristic and an
  indicate characteristic for responses.
- Requests and responses are JSON inside a small frame (constant 16-byte token, u32
  length), fragmented into 148-byte writes. The handshake subscribes to 24
  properties; the printer answers with the cartridge width and part number,
  remaining percentage, battery class, firmware, error flags and the last job's
  outcome. Feed and cut are property writes.
- A print job is an STX-prefixed command stream carrying the job id, copies, cut
  option and canvas size, followed by the raster: columns sent right-to-left, each as
  literal bytes or run lengths, whichever is shorter. The canvas is the supply width
  at 300 dpi (450 rows on 1.5 in tape, 106 on the 0.355 in sleeve).
- The printer identifies a cartridge only by part number; the name, dpi and die-cut
  flag come from a small table in `src/supply.rs` covering the cartridges seen so far.

## Usage

```
cargo build --release
brusdk scan                      # find the printer (press its power button first; scans can take up to 60 s)
brusdk gatt                      # dump services and characteristics
brusdk status                    # connect, handshake, print every property and the derived status
brusdk print label.png           # encode only; add --send to print, --copies N, --cut 0|1|2, --row R, --col C
brusdk feed
brusdk cut
brusdk serve --port 5178         # sidecar-compatible HTTP API for the label studio
brusdk probes --out dir --width 150 [--rows 107]
cargo test
```

`label.png` is a black-on-white image, rows across the tape and columns along it;
pixels darker than 128 print. The studio sends 432-row images (its 1.44 in window),
which land at the top of the canvas.

### As a library

The binary is a thin layer over the crate. `examples/print.rs` is the whole flow:

```rust
let adapter = brusdk::ble::adapter().await?;
let mut printer = Printer::connect(&adapter, Duration::from_secs(60)).await?;   // scan, connect, handshake
let status = printer.status();                                                  // width, part number, remaining %, flags
let cartridge = supply::lookup(&status.supply_part_number.unwrap_or_default()).unwrap();
let raster = Raster::place(&img, supply::canvas_rows(cartridge.width_in), 0, 0); // image onto the canvas
let mut job = JobParams::new(new_job_id(), supply::job_prefix(cartridge.name));
job.copies = 2;
job.cut = 1;
let result = printer.print(&job, &raster).await?;                                // waits for "<id>:Successful"
printer.feed().await?;
printer.disconnect().await?;
```

`job::encode` and `job::fragments` are pure functions if you only want the bytes, and
`proto::Reassembler` parses the printer's indication stream.

### HTTP API (`brusdk serve`)

Same routes and JSON shapes as the studio's Node sidecar: `GET /api/printer`,
`POST /api/connect {scanSeconds}`, `/api/disconnect`, `/api/print {pngBase64, copies,
cutOption, xOffsetIn?, yOffsetIn?}`, `/api/dryrun`, `/api/feed`, `/api/cut`.
`xOffsetIn` keeps the SDK's convention: the SDK placed images 2 × (width − 1.44 in)
down the canvas, and the studio passes the negative of that to cancel it. The
label-server only spawns the Node sidecar when nothing listens on its sidecar port,
so running `brusdk serve` on that port is enough to switch the studio over.

## Layout

- `examples/print.rs` library usage end to end.
- `src/job.rs` job encoder and fragmentation; `src/proto.rs` request framing,
  properties, response reassembly; `src/printer.rs` BLE session (btleplug);
  `src/server.rs` HTTP surface; `src/supply.rs` cartridge table and canvas rules;
  `src/probes.rs` probe image generator.
- `tests/captured_jobs.rs` byte-exact comparison against the captures.
- `tools/capture/` the capture harness: `hook.mjs` (polyfill hook), `drive.mjs` (runs
  the SDK as a black box), `gatt-dump.mjs`, `decode.py`, `hci-compare.py`.
- `captures/` raw sessions with a README each, probe images and the SDK's rasters.
- `PROTOCOL.md`, `LEGAL.md`, `STATUS.md` (what is known, guessed and open).

## Cartridges

The printer only tells us a cartridge's part number (property `004D`), width and
remaining percentage. Its name, dpi and die-cut flag are not on the wire, so they
come from the table in `src/supply.rs`, which holds the six cartridges seen so far:

| Part number | Name | Width |
|---|---|---|
| 5153508 | M5C-1500-595-WT-BK | 1.5 in |
| 5072987 | M5C-1500-595-OR-BK | 1.5 in |
| 5072986 | M5C-1500-595-CL-BK | 1.5 in |
| 5072905 | M5C-1500-595-CL-WT | 1.5 in |
| 5072903 | M5C-1500-595-BK-WT | 1.5 in |
| 5073028 | M4C-250-7641-YL | 0.355 in |

Any other cartridge is refused with `unknown supply part number`. To add one: insert
it, run `brusdk status`, take `supply_part_number` and `supply_width_in` from the
output, read the name off the cartridge label, and append a line to `SUPPLIES` in
`src/supply.rs`:

```rust
Supply { part_number: "5073028", name: "M4C-250-7641-YL", width_in: 0.355, die_cut: false },
```

The name matters: its first 10 characters go into every job header. Die-cut
supplies have not been seen yet, so `die_cut: true` is untested.

## Known limits

- The printer rejects all-white jobs.
- Battery is reported as a class; only "High" has been seen (shown as 100 %).
- Ten of the printer's boolean flags and two numeric properties are still unmapped;
  the latch-open flags are known.
