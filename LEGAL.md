# Legal method statement

This repository contains an independent, clean-room implementation of the
Bluetooth Low Energy protocol spoken by the Brady M511 label printer.

## What was and was not done

Brady's Web SDK (`@bradycorporation/brady-web-sdk` 3.2.2) is licensed under a
proprietary Software License Agreement. Section 2b of that agreement forbids
translating, disassembling, de-compiling or reverse engineering *the Software*
(the SDK's object code). Nothing in this repository was produced by doing any
of those things:

- No file of the SDK package was opened, read, grepped, pretty-printed,
  decompiled or otherwise inspected. That includes `dist/bundle.js`, the
  TypeScript type definitions, source maps and any documentation shipped in
  the package. The only SDK file read was `LICENSE.md`, to establish this
  method.
- No identifier, constant, string, symbol name or code structure from the SDK
  was copied or used. Names in this repository were chosen from what the
  bytes on the wire look like, or from standard Bluetooth terminology.
- Every protocol fact in `PROTOCOL.md` carries a provenance line pointing at a
  capture file in `captures/` and the byte offsets that support it. Facts that
  could not be established from the wire are marked as unknown rather than
  guessed from anything else.

## What the observations are

Two kinds of observation were used, both of which are ordinary interoperability
work on communications and on the device itself, not on the SDK:

1. **Wire captures.** The SDK, running unmodified inside our own tooling, talks
   to the printer through the open-source `webbluetooth` polyfill (MIT). A
   small opt-in hook in *our* code (`tools/capture/hook.mjs`) records the bytes
   that cross the polyfill's characteristic boundary: what was written to which
   GATT characteristic, and what the printer sent back, with timestamps. This
   is the same information a Bluetooth sniffer sees. Where possible the same
   sessions were also recorded with Apple's PacketLogger (raw HCI) to confirm
   the hook shows the complete byte stream.
2. **Device enumeration.** The printer's advertisement and GATT database
   (services, characteristics, properties, descriptors, readable values) were
   enumerated directly with a generic BLE client (`tools/capture/gatt-dump.mjs`
   and the Rust crate). No SDK code is involved.

Ground-truth images for print captures come from our own renderer, so the
input image is known pixel-for-pixel and the encoding can be derived by
comparing images that differ in one known way.

## Result

The Rust crate in this repository is an original work written from
`PROTOCOL.md`. It does not link to, embed, or depend on the Brady SDK.
