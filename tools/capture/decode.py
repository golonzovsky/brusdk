#!/usr/bin/env python3
"""Reassemble and pretty-print a capture jsonl: tx frames per characteristic, rx indications."""
import json, sys, binascii
def main(path):
    tx = {}  # char -> list of (t, seqhdr, payload)
    rxbuf = b''; rxstart = None
    for line in open(path):
        r = json.loads(line)
        ev = r['ev']
        if ev == 'tx':
            b = bytes.fromhex(r['hex'])
            print(f"{r['t_ms']:>9.1f} TX {r['char']} {r['mode']} len={len(b)} hdr={b[:3].hex()} body[0:24]={b[3:27].hex()}")
            tx.setdefault(r['char'], []).append((r['t_ms'], b[:3], b[3:]))
        elif ev == 'rx':
            b = bytes.fromhex(r['hex'])
            print(f"{r['t_ms']:>9.1f} RX {r['char']} len={len(b)} first={b[:20].hex()}")
            if rxstart is None: rxstart = r['t_ms']
            rxbuf += b
        elif ev in ('tx-done',):
            print(f"{r['t_ms']:>9.1f} tx-done {r['took_ms']}ms")
        else:
            print(f"{r['t_ms']:>9.1f} {ev} {json.dumps({k:v for k,v in r.items() if k not in ('t_ms','ev','file','argv')})[:160]}")
    print("\n==== TX reassembled ====")
    for ch, frags in tx.items():
        body = b''.join(p for _, _, p in frags)
        print(f"-- {ch}: {len(frags)} fragments, {len(body)} bytes; seq headers {[h.hex() for _,h,_ in frags]}")
        dump(body)
    print("\n==== RX concatenated ====")
    dump(rxbuf)
def dump(body):
    i = 0
    while i < len(body):
        # frame: 16-byte token, 4-byte LE length, payload
        if len(body) - i < 20: print("  trailing", body[i:].hex()); break
        tok = body[i:i+16]; ln = int.from_bytes(body[i+16:i+20], 'little'); payload = body[i+20:i+20+ln]
        print(f"  token={tok.hex()} len={ln} (have {len(payload)})")
        try: print("  " + payload.decode('utf-8'))
        except UnicodeDecodeError: print("  (binary) " + payload[:64].hex() + ("..." if ln > 64 else ""))
        i += 20 + ln
if __name__ == '__main__': main(sys.argv[1])
