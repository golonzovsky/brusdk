# Session 3, 2026-09-08 (first prints from the Rust implementation, no SDK)

Cartridge: M5C-1500-595-WT-BK (part 5153508), 86 % remaining.

| File | What |
|---|---|
| `rust-band-r100-200-150.log` | `brusdk print captures/probes/band-r100-200-150.png --send`: 738 bytes, 5 fragments, printer answered `<job id>:Successful` |

| `rust-feed.log`, `rust-cut.log` | `brusdk feed`, `brusdk cut`: set requests 0007/0004 written, no error; properties afterwards show 0029 empty |

Alex confirmed the Rust band print looks the same as the SDK's print of the same file.

`serve.log`: `brusdk serve --port 5178` replacing the Node sidecar; Alex connected and printed a
label from the studio UI through it ("worked well").

Cartridge swap (latch open, black-on-white out, black-on-orange in) while the studio was connected
through `brusdk serve`: `latch-open-snapshot.json` (0006 and 0066 True), `orange-cartridge-snapshot.json`
(part 5072987, 96 %, flags clear), `*orange-info.jsonl` + `orange-info.log` (SDK name for that part).
`clear-status.log`: `brusdk status` with the black-on-clear cartridge (part 5072986, 83 %).
`clear-white-status.log`: `brusdk status` with the white-on-clear cartridge (part 5072905, 94 %).
`black-white-status.log`: `brusdk status` with the white-on-black cartridge (part 5072903, 94 %).
`m4c-250-status.log`, `*m4c-250-info.jsonl`, `m4c-250-info.log`: M4C-250-7641-YL sleeve (part 5073028, 0.355 in, 92 %), Rust and SDK views.
