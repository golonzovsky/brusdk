# Brady M511 BLE protocol (as observed on the wire)

Every fact below cites a capture in `captures/` as `file @ event-time-ms [byte range]`.
Bytes are hex. Unknowns are marked **UNKNOWN**. Nothing here comes from Brady's SDK
code; see `LEGAL.md`.

## 1. Advertisement and GATT

- Local name `M511-PGM5112608303011` (btleplug shows the short name `M511` in the
  scan response; the polyfill reports `M511`). Advertised service list: `0xFD1C`.
  Manufacturer data, company id `0x066A` (1642):
  `0d 01 00 00 5a 00 00 00 00 00 00 00 00 00 96 00 00` (17 bytes). Meaning **UNKNOWN**
  (`5a` = 90 matches the supply-remaining percent reported later; not confirmed).
  Tx power level 0, RSSI about -63 dBm at a metre.
  Provenance: `captures/2026-09-08-s0/gatt-rust.txt`, `gatt-node.json`.
- One primary service `0000fd1c-0000-1000-8000-00805f9b34fb` with three
  characteristics and nothing else (no Device Information, no Battery service):

  | UUID | Properties | Descriptors | Role (observed) |
  |---|---|---|---|
  | `7d9d9a4d-b530-4d13-8d61-e0ff445add19` | write, write-without-response | none | print job data (not yet captured) |
  | `a61ae408-3273-420c-a9db-0669f4f23b69` | write, write-without-response | none | requests (property subscribe) |
  | `786af345-1b68-c594-c643-e2867da117e3` | indicate | `0x2902` CCCD | responses / property updates |

  Provenance: `gatt-node.json` (polyfill, no SDK) and `gatt-rust.txt` (btleplug) agree.
- ATT view (`captures/2026-09-08-s0/hci.pklg`, PacketLogger, decoded with tshark):
  MTU is negotiated to 156 on every connection (client and server both offer 156),
  so the largest ATT write value is 153 bytes; the SDK's 151-byte fragments and the
  printer's 153-byte indications are single ATT PDUs, nothing is split below the
  polyfill. Handles: `0x000e` = `7d9d9a4d` (job), `0x0010` = `a61ae408` (request),
  `0x0012` = `786af345` (status), `0x0013` = its CCCD (written `02 00` to enable
  indications). The HCI trace also shows the GAP/GATT services CoreBluetooth hides
  (`0x2a00` device name at `0x0007`, `0x2a05` service-changed at `0x0003` with CCCD
  `0x0004`). Every one of the 204 hook write records taken before the .pklg was saved
  appears byte-identical on the expected handle, and all 16 sessions' indication
  streams appear contiguously (`tools/capture/hci-compare.py`), so the hook is a
  faithful record of the link.
- Subscribing to the indicate characteristic alone produces no traffic: 15 s of
  silence after enabling indications with no request written
  (`gatt-node.json` events: only `connected`, `subscribed`, `disconnect-requested`).

## 2. Framing

### 2.1 Message (both directions)

```
+----------------+----------------+------------------------+
| token[16]      | length u32 LE  | payload[length]        |
+----------------+----------------+------------------------+
```

- `token` = `96 c2 f7 4a 1d 21 42 32 86 78 20 ef e9 7b c2 d3` in every message seen so
  far, in both directions and across two separate connections. It is therefore a
  constant, not a per-session nonce. Whether the printer accepts a different value is
  **UNKNOWN**.
  Provenance: `2026-09-08-s0/*idle60.jsonl @19867.8 [3..19]`, `@20252.3 [0..16]`;
  `*sleepwatch.jsonl @28397.1 [3..19]`.
- `length` counts only the payload. `8a 01 00 00` = 394 for the subscribe request;
  `ea 04 00 00` = 1258 for the response. (`idle60 @19867.8 [19..23]`, `@20252.3 [16..20]`)
- Payload is UTF-8 JSON text. No terminator, no checksum seen.

### 2.2 Client to printer fragmentation (writes)

Messages are written to `a61ae408` as write-with-response fragments of at most
151 bytes. Each fragment carries a 3-byte header before the message bytes:

```
byte 0: 01 for a fragment with more to follow, 03 for the last fragment
byte 1: fragment index, starting at 00
byte 2: 00
```

Observed for the 414-byte subscribe message: fragments of 151, 151, 121 bytes with
headers `01 00 00`, `01 01 00`, `03 02 00` (`idle60 @19867.8, @19988.9, @20122.0`).
Each write-with-response completes in about 120-130 ms (`tx-done took_ms`).
Whether byte 0 is a bit field (bit0 = data, bit1 = last) or an enum is **UNKNOWN**
until a single-fragment message is seen. Whether the 148-byte payload cap is the
SDK's choice or a printer limit is **UNKNOWN** (needs a larger-MTU test).

### 2.3 Printer to client (indications)

The printer sends the message bytes as a plain stream of indications on `786af345`
with no per-fragment header; the message header (token + length) appears once at the
start and the receiver reassembles by `length`. Indication sizes alternate 153, 94
bytes (247 per pair) with a 43-byte tail: 5 × 247 + 43 = 1278 = 20 + 1258.
Indications arrive about 120-190 ms apart. (`idle60 @20252.3 .. @21964.7`)

## 3. Property protocol (JSON)

### 3.1 Subscribe request

Written by the client right after enabling indications, before anything else:

```json
{"PropertySubscribeRequests":[{"ID": "0006"},{"ID": "0005"},...,{"ID": "0020" }]}
```

24 IDs in this order: 0006 0005 000A 001C 0021 0027 0025 0009 0029 0001 0024 0026
000C 000D 000E 000F 0013 0016 0066 0061 005A 004D 001F 0020. The irregular spacing
inside the JSON is what was on the wire. (`idle60 @19867.8`)

### 3.2 Response

The printer answers with `PropertyGetResponses`, one entry per requested ID, and
then sends the identical message a second time about 1.9 s later (two responses to
one request; second one is presumably the first "subscription update"):

```json
{"PropertyGetResponses":[{"ID":"0006","Value":"False","Status":"Successful"}, ...]}
```

Values as strings. Observed with cartridge M5C-1500-595-WT-BK (part 5153508),
battery reported 100 % by the SDK, no AC:

| ID | Value | Interpretation (correlated with the SDK's reported state) |
|---|---|---|
| 0001 | `High` | battery level class; the SDK reported 100 % while this read `High` (sessions 0-3, battery full). Other class names and their percentages **UNKNOWN** until seen |
| 000C | `1500` | supply width in mils (SDK: supplyWidth 1.5 in). M4C-250-7641-YL sleeve: `355` = 0.355 in, SDK agrees (`2026-09-08-s3/m4c-250-status.log`, `m4c-250-info.log`) |
| 000D | `206` | **UNKNOWN**; `236` on the M4C-250 sleeve, so it depends on the supply type (not width in mils, not the SDK's supplyHeight) |
| 000E | `0` | **UNKNOWN**; `500` on the M4C-250 sleeve while the SDK reported supplyHeight 0 for it and 0.5 for the 1.5 in tapes (so it is not what the SDK calls supplyHeight either) |
| 000F | `0` | **UNKNOWN** (offset?) |
| 0016 | `90` | supply remaining percent (SDK: 90) |
| 0020 | `2.0.1005769` | firmware version |
| 0026 | `30` | **UNKNOWN** |
| 0029 | `` (empty) | **UNKNOWN** (message text?) |
| 001F | `True` | **UNKNOWN** |
| 004D | `5153508` | supply part number (SDK: supplyYNumber). Second cartridge: `5072987` = SDK name `M5C-1500-595-OR-BK`, 96 % (`2026-09-08-s3/*orange-info.jsonl`, `orange-info.log`). Third: `5072986` = `M5C-1500-595-CL-BK` (name read off the cartridge by Alex), 83 % (`clear-status.log`). Fourth: `5072905` = `M5C-1500-595-CL-WT` (same), 94 % (`clear-white-status.log`). Fifth: `5072903` = `M5C-1500-595-BK-WT` (same), 94 % (`black-white-status.log`). Sixth: `5073028` = SDK name `M4C-250-7641-YL`, 92 % (`m4c-250-info.log`) |
| 0006, 0066 | `True` while the cartridge latch is open (printer LED blinking), `False` otherwise | cartridge latch / cover open (`2026-09-08-s3/latch-open-snapshot.json`, live property updates via `brusdk serve`); which of the two is latch vs. cover, and whether one means "cartridge not readable", **UNKNOWN** |
| 0005 000A 0013 001C 0021 0024 0025 0027 005A 0061 | `False` | status flags, all clear so far; meaning **UNKNOWN** until seen True |

Provenance: `idle60 @20252.3..@21964.7` (reassembled with `tools/capture/decode.py`).

Not on the wire: the supply *name* (`M5C-1500-595-WT-BK-BULK`), dpi (300), die-cut
flag. The SDK derives these from the part number; a clean implementation needs its
own part-number table (only entries Alex's cartridges provide).

## 4. Print job

Provenance for this whole section: `captures/2026-09-08-s1/*.jsonl` (11 probe prints,
150 x 432 input images, cartridge part 5153508 = M5C-1500-595-WT-BK), decoded with
`tools/capture/decode.py`; the per-probe raster sections are listed in
`captures/2026-09-08-s1/README.md`.

### 4.1 Transport

The job is written to `7d9d9a4d` as write-with-response fragments of at most 151
bytes: 3-byte header + up to 148 job bytes. Header byte 0 is `01` for an ordinary
fragment, `02` on every 16th fragment (`idx % 16 == 15`: `02 0f 00`, `02 1f 00`,
`02 2f 00` in `altrows-150`), `03` on the last; byte 1 = `idx` from 0; byte 2 = 0
(58 fragments, last `03 39 00`). The `02` fragments are acked no slower than the
others, so what the marker is for is **UNKNOWN** (a block boundary of 16 x 148 bytes?).
Unlike requests there is **no** token/length envelope.
Each fragment is acked in 110-200 ms, so an 8.4 kB job takes about 7.4 s to send.
About 4.45 s after the last fragment's ack the printer pushes a property update (4.4).

### 4.2 Command stream

`02` (STX) introduces a one-letter command. `K` commands carry a big-endian 16-bit
subcommand. String arguments end with `0d` (CR). Numeric arguments are signed decimal
ASCII. Example, dot-r0c0-150 (144 bytes):

```
02 4b 00 0a  "b8c4e834a8384349ad74f6ffdcc13f16" 0d   K/000a  job id: 32 lowercase hex = a UUIDv4 without dashes, new per job
02 4b 00 09  "M5C-1500-5" 0d                        K/0009  first 10 characters of the supply name (from the SDK's part table)
02 44 "+0001"                                       D       UNKNOWN, stayed +0001 with copies=2
02 43 "+0001"                                       C       number of copies (`s2/copies2`: +0002)
02 63 00                                            c       UNKNOWN (unchanged by copies, cut option, offsets)
02 70 "+00"  02 6f "+00"  02 4f "+00"  02 62 "+00"  p o O b UNKNOWN; unchanged by the SDK's x/y offsets (those move the raster, 4.5)
02 4d 00                                            M       cut option: 00 = cut at end of job, 01 = after each label, 02 = never (`s2/cut1`, `s2/cut2`)
02 4b 00 0c  "0450" "0150"                          K/000c  raster height (rows across the tape) and width (columns along it), %04d each
02 41  02 51  02 61                                 A Q a   UNKNOWN
02 49 "BUlbl0" 0d                                   I       label name, constant so far
58 00 00  59 00 00                                  X=0 Y=0 cursors (u16 LE)
<column data, 4.3>
59 96 00                                            Y=150 (= width), emitted when at least one column was sent and the cursor is not at width yet (absent in the all-white jobs)
ff ff 0d                                            end of raster
02 61  02 47  02 41  02 4b 00 0b                    a G A K/000b   UNKNOWN (G is presumably "go")
```

With copies > 1 the header (`K/000a` .. `M`) is sent once and the label block from
`K/000a` on (`K/000a` id again, then `K/000c` .. `K/000b`, without `K/0009`, `D`, `C`,
`c`, `p/o/O/b`, `M`) is repeated per copy with the same job id (`s2/copies2`: 233 bytes =
144 + 89). The printer then reports one `<job id>:Successful`.

Height is 450 = supply width 1500 mils at 300 dpi; the SDK draws the 432-row input at
the top of a 450-row canvas (`*-raster.png`, all 150 x 450, ink at the same pixels as
the input). Width is the image width in pixels.

### 4.3 Raster encoding

The raster is sent column by column (a column = one line across the tape, 450 rows =
57 bytes, row 0 first, MSB first). `Y` is the column cursor; column `Y` holds image
column `x = width - 1 - Y`, so the image's right-most column is sent first
(`dot-r0c0` -> `59 95 00` = 149; `dot-r0c1` -> `59 94 00` = 148; `dot-r431cLast`
x=149 -> Y=0, no cursor move). After a column's data the cursor advances by one.
Blank columns are skipped with an explicit `59 <Y>` before the next non-blank column
(`dot-r0c1`: `59 94 00 ... 59 96 00`).

Two column encodings, each `op count payload` with `count` = payload bytes (1..255?):

- `80 n b1..bn`: literal bitmap bytes, trailing zero bytes trimmed
  (`dot-r0c0`: `80 01 80`; `dot-r1c0`: `80 01 40`; `dot-r8c0`: `80 02 00 80`;
  `altrows`: `80 36 aa*54` = 54 bytes, rows 432..449 trimmed).
- `81 n r1..rn`: run lengths over rows from the top, each byte = colour bit 7
  (1 = black) | (length - 1) in bits 0-6, so a run is 1..128 rows; longer runs chain;
  the trailing white run is omitted
  (`row100`: `81 02 63 80` = white 100, black 1; `band-r100-200`: `81 02 63 e3` =
  white 100, black 100; `col5`: `81 04 ff ff ff af` = black 432;
  `dot-r431cLast`: `81 05 7f 7f 7f 2e 80` = white 431, black 1).

The SDK picks the literal form unless run-length is strictly shorter (ties go to
literal: `dot-r0c0` 1 vs 1 byte, `dot-r8c0` 2 vs 2). Whether the printer accepts either
form for any column is assumed, not tested. `src/job.rs` reproduces all 11 captured
jobs byte for byte, fragmentation included (`cargo test`, `tests/captured_jobs.rs`).

`X` (`58`) was always 0; presumably a row offset. `ff ff` terminates the raster.

### 4.5 Offsets

The SDK's x/y offsets never reach the header; they move the image on the canvas:

- x offset (across the tape) shifts rows: with x = 0 the dot at input (0,0) lands on
  row 36 (`s2/x0`: column data `81 02 23 80` = white 36, black 1); with x = -0.12 in
  (36 px) it lands on row 0. So the SDK's default placement is 0.12 in below the top of
  the 450-row canvas, and the label studio's -0.12 in cancels that.
- y offset (along the tape) shifts columns: y = 0.1 in moves the dot from column 149
  to 119, i.e. 30 px toward the far end (`s2/y0.1`: `59 77 00`), then `59 96 00`.
- Odd width works the same: 151 px -> `K/000c "0450" "0151"`, corner dots at Y = 0 and
  Y = 150 (`s2/corners-p1-150`).

### 4.4 Printer response to a job

- Accepted job: about 4.45 s after the last fragment ack, an indication with
  `{"PropertyGetResponses":[{"ID":"0029","Value":"<job id>:Successful",...},{"ID":"001F","Value":"True",...}]}`;
  when the supply percentage changes it is included (`0016` 90 -> 88 after `altrows`).
- Rejected job: an all-white raster (`X 0 Y 0 ff ff`, both `white-64` and `white-150`)
  makes the printer answer `0009: True`, `0029: "Unknown:Failed"` about 2 s after the
  ack and print nothing. `0009` stays True and `0029` keeps the text in the next
  connection's property list until a job succeeds. So a blank job is invalid, and
  `0029` = last job status text, `0009` = last job failed.
- Between the job ack and the update the printer re-sends the full 24-property list once.

## 5. Feed / cut

Both are property writes on `a61ae408`, framed like every request (token + length +
JSON, one fragment `03 00 00`), sent after the normal handshake:

- Feed: `{"PropertySetRequests":[{"ID": "0007", "Value": "True"}]}` (57-byte payload;
  `s2/*feed.jsonl @36261`). The printer fed about half an inch of tape.
- Cut: `{"PropertySetRequests":[{"ID": "0004", "Value": "True"}]}` (`s2/*cut.jsonl @34468`).

No response was captured within the 3 s the driver waited before disconnecting; whether
the printer acknowledges a set request is **UNKNOWN**.

## 6. Idle, sleep, disconnect

- 60 s idle after the handshake: no traffic in either direction (`idle60`).
- Client disconnect: plain GATT disconnect, no message first (`idle60 @82101.8`).
