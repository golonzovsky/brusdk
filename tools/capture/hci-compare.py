#!/usr/bin/env python3
"""Cross-check hook captures against a PacketLogger .pklg: every hook write must appear as an ATT
write on the expected handle, and every session's indication bytes must appear contiguously.
  tshark -r hci.pklg -Y "btatt.opcode == 0x12 || btatt.opcode == 0x1d || btatt.opcode == 0x52" \
     -T fields -e frame.time_epoch -e btatt.opcode -e btatt.handle -e btatt.value > hci-all.tsv
  hci-compare.py hci-all.tsv captures/2026-09-08-s*/*.jsonl"""
import sys, json
rows = [l.split('\t') for l in open(sys.argv[1]).read().splitlines()]
writes = {}
for t, op, h, v in rows:
    if op in ('0x12', '0x52') and v:
        writes.setdefault(v, set()).add(h)
ind_stream = ''.join(v for t, op, h, v in rows if op == '0x1d' and h == '0x0012')
handles = {'7d9d9a4d': '0x000e', 'a61ae408': '0x0010'}
tot = miss = bad = rx_ok = rx_tot = 0
for f in sys.argv[2:]:
    rx = ''
    for l in open(f):
        r = json.loads(l)
        if r['ev'] == 'tx':
            tot += 1
            if r['hex'] not in writes: miss += 1; print('missing tx', f, r['hex'][:24])
            elif handles[r['char']] not in writes[r['hex']]: bad += 1; print('wrong handle', f, writes[r['hex']])
        if r['ev'] == 'rx': rx += r['hex']
    if rx:
        rx_tot += 1
        if rx in ind_stream: rx_ok += 1
        else: print('rx not contiguous in HCI', f, len(rx) // 2)
print(f'tx {tot} records: {miss} missing, {bad} on the wrong handle; rx sessions {rx_ok}/{rx_tot} found contiguously')
