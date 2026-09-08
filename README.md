# brusdk

Clean-room Rust driver for the Brady M511 label printer over Bluetooth LE, written from
wire captures only (see `LEGAL.md`, `PROTOCOL.md`, `STATUS.md`).

```
cargo build --release
brusdk scan                      # find the printer (press its power button first)
brusdk gatt                      # dump services/characteristics
brusdk status                    # connect, handshake, print all properties
brusdk print label.png           # encode only; --send to print; --copies N --cut 0|1|2 --row R --col C
brusdk feed | brusdk cut
brusdk serve --port 5178         # sidecar-compatible HTTP API for the label studio
brusdk probes --out dir --width 150
cargo test                       # byte-exact checks against captures/
```

`label.png` is a 1-bit (or grayscale, thresholded at 128) image, rows across the tape,
columns along it; 432 rows is the studio's 1.44 in window, the canvas is 450 rows.
Layout: `src/job.rs` job encoder, `src/proto.rs` request framing and properties,
`src/printer.rs` BLE session, `src/server.rs` HTTP surface, `src/supply.rs` part table,
`tools/capture/` the capture harness, `captures/` raw sessions with READMEs.
