# Session 2, 2026-09-08 (job options, feed, cut)

Cartridge: M5C-1500-595-WT-BK (part 5153508), 88 % -> 86 % remaining. Same driver
and probe images as session 1 (`../probes/`). All jobs accepted and printed.

| Label | Command | Finding |
|---|---|---|
| copies2 | print dot-r0c0-150 --copies 2 | `C+0002`; label block repeated with the same job id |
| cut1 | print dot-r0c0-150 --cut 1 | `M 01` |
| cut2 | print dot-r0c0-150 --cut 2 | `M 02` |
| x0 | print dot-r0c0-150 --x 0 | dot moved to row 36; header unchanged |
| y0.1 | print dot-r0c0-150 --y 0.1 | dot moved to column 119 (30 px); header unchanged |
| corners-p1-150 | print corners-p1-150 (151 px wide) | width field 0151, columns 0 and 150 |
| feed | feed | `PropertySetRequests` ID 0007 = True |
| cut | cut | `PropertySetRequests` ID 0004 = True |
