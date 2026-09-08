# Session 3, 2026-09-08 (first prints from the Rust implementation, no SDK)

Cartridge: M5C-1500-595-WT-BK (part 5153508), 86 % remaining.

| File | What |
|---|---|
| `rust-band-r100-200-150.log` | `brusdk print captures/probes/band-r100-200-150.png --send`: 738 bytes, 5 fragments, printer answered `<job id>:Successful` |

| `rust-feed.log`, `rust-cut.log` | `brusdk feed`, `brusdk cut`: set requests 0007/0004 written, no error; properties afterwards show 0029 empty |

Alex confirmed the Rust band print looks the same as the SDK's print of the same file.

`serve.log`: `brusdk serve --port 5178` replacing the Node sidecar; Alex connected and printed a
label from the studio UI through it ("worked well").
