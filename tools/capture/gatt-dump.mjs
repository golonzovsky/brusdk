// Generic BLE enumeration of the printer with the webbluetooth polyfill only (no SDK).
//   node gatt-dump.mjs [--scan 30] [--listen 15] [--out captures/xxx/gatt.json]
// Connects to the first device whose name starts with M511, lists every service,
// characteristic (with properties) and descriptor, reads what is readable,
// subscribes to every notify/indicate characteristic and records what arrives
// unsolicited for --listen seconds, then disconnects.
import fs from 'node:fs';
import { Bluetooth } from 'webbluetooth';

const argv = process.argv.slice(2);
const arg = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const scan = +arg('--scan', 30);
const listen = +arg('--listen', 15);
const out = arg('--out', 'gatt.json');
const t0 = performance.now();
const now = () => +(performance.now() - t0).toFixed(1);
const hex = (v) => Buffer.from(v.buffer, v.byteOffset, v.byteLength).toString('hex');
const ascii = (v) => { const b = Buffer.from(v.buffer, v.byteOffset, v.byteLength); return /^[\x20-\x7e]*$/.test(b.toString('latin1')) ? b.toString('latin1') : undefined; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const bt = new Bluetooth({ scanTime: scan, deviceFound: (d) => { console.error(`[scan] ${d.name ?? '?'} ${d.id}`); return d.name?.startsWith('M511') === true; } });
console.error(`[scan] up to ${scan}s for M511*`);
const dev = await bt.requestDevice({ filters: [{ namePrefix: 'M511' }] });
const ad = dev._adData ?? {};
const mapHex = (m) => Object.fromEntries([...(m ?? new Map())].map(([k, v]) => [String(k), hex(v)]));
const dump = { at: new Date().toISOString(), device: { name: dev.name, id: dev.id, rssi: ad.rssi, txPower: ad.txPower, manufacturerData: mapHex(ad.manufacturerData), serviceData: mapHex(ad.serviceData) }, services: [], events: [] };
dev.addEventListener('gattserverdisconnected', () => dump.events.push({ t_ms: now(), ev: 'disconnected' }));
await dev.gatt.connect();
dump.events.push({ t_ms: now(), ev: 'connected' });

for (const svc of await dev.gatt.getPrimaryServices()) {
  const s = { uuid: svc.uuid, characteristics: [] };
  dump.services.push(s);
  for (const ch of await svc.getCharacteristics()) {
    const c = { uuid: ch.uuid, properties: Object.entries(ch.properties).filter(([, v]) => v).map(([k]) => k), descriptors: [] };
    s.characteristics.push(c);
    try {
      for (const d of await ch.getDescriptors()) {
        const rec = { uuid: d.uuid };
        try { const v = await d.readValue(); rec.hex = hex(v); rec.ascii = ascii(v); } catch (e) { rec.error = e.message; }
        c.descriptors.push(rec);
      }
    } catch (e) { c.descriptorsError = e.message; }
    if (ch.properties.read) {
      try { const v = await ch.readValue(); c.value = { hex: hex(v), ascii: ascii(v) }; } catch (e) { c.readError = e.message; }
    }
    if (ch.properties.notify || ch.properties.indicate) {
      ch.addEventListener('characteristicvaluechanged', () => dump.events.push({ t_ms: now(), ev: 'rx', char: ch.uuid, hex: hex(ch.value) }));
      try { await ch.startNotifications(); dump.events.push({ t_ms: now(), ev: 'subscribed', char: ch.uuid }); } catch (e) { c.notifyError = e.message; }
    }
  }
}
console.error(JSON.stringify(dump.services, null, 1));
console.error(`[listen] ${listen}s for unsolicited data`);
await sleep(listen * 1000);
try { dev.gatt.disconnect(); } catch {}
dump.events.push({ t_ms: now(), ev: 'disconnect-requested' });
await sleep(500);
fs.writeFileSync(out, JSON.stringify(dump, null, 1));
console.error(`[done] ${dump.events.length} events -> ${out}`);
process.exit(0);
