# Session 1, 2026-09-08 (encoding probes)

Cartridge: M5C-1500-595-WT-BK (part 5153508), 90 % -> 88 % remaining, battery 100 %.
Input images from `brusdk probes --width 150` (150 x 432 px, 1-bit PNG) in `../probes/`;
`<name>-raster.png` is the 150 x 450 canvas the SDK rasterised for that print.
Printed once each through `tools/capture/cap.sh` with copies=1, cut=0, x=-0.12, y=0.
`white-64` and `white-150` were rejected by the printer (nothing printed); the rest printed.
Two `NOT CONNECTED` retries (scan timeouts) left extra empty jsonl files, removed.

| Probe | Job bytes | Fragments | Raster section |
|---|---|---|---|
| white-64 | 138 | 1 | `58 00 00 59 00 00 ff ff 0d` |
| white-150 | 138 | 1 | `58 00 00 59 00 00 ff ff 0d` |
| dot-r0c0-150 | 144 | 1 | `58 00 00 59 00 00 59 95 00 80 01 80 ff ff 0d` |
| dot-r1c0-150 | 144 | 1 | `58 00 00 59 00 00 59 95 00 80 01 40 ff ff 0d` |
| dot-r0c1-150 | 147 | 1 | `58 00 00 59 00 00 59 94 00 80 01 80 59 96 00 ff ff 0d` |
| dot-r8c0-150 | 145 | 1 | `58 00 00 59 00 00 59 95 00 80 02 00 80 ff ff 0d` |
| dot-r431cLast-150 | 148 | 1 | `58 00 00 59 00 00 81 05 7f 7f 7f 2e 80 59 96 00 ff ff 0d` |
| col5-150 | 150 | 2 | `58 00 00 59 00 00 59 90 00 81 04 ff ff ff af 59 96 00 ff ff 0d` |
| row100-150 | 738 | 5 | `58 00 00 59 00 00 81 02 63 80 81 02 63 80 81 02 63 80 81 02 63 80 81 02 63 80 81 02 63 80 81 02 63 80 81 02 63 80 81 02 ...` |
| altrows-150 | 8538 | 58 | `58 00 00 59 00 00 80 36 aa aa aa aa aa aa aa aa aa aa aa aa aa aa aa aa aa aa aa aa aa aa aa aa aa aa aa aa aa aa aa aa ...` |
| band-r100-200-150 | 738 | 5 | `58 00 00 59 00 00 81 02 63 e3 81 02 63 e3 81 02 63 e3 81 02 63 e3 81 02 63 e3 81 02 63 e3 81 02 63 e3 81 02 63 e3 81 02 ...` |
| checker-150 | 8538 | 58 | `58 00 00 59 00 00 80 36 55 55 55 55 55 55 55 55 55 55 55 55 55 55 55 55 55 55 55 55 55 55 55 55 55 55 55 55 55 55 55 55 ...` |
