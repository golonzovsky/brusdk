// Opt-in wire capture at the webbluetooth polyfill boundary.
//   BRADY_CAPTURE=captures/2026-09-08-probe node --import /path/to/hook.mjs brady.mjs print ...
// Records every characteristic write, read, notification/indication and GATT
// discovery step as one JSON line: {t_ms, ev, char, hex, ...}. Off unless
// BRADY_CAPTURE is set. Patches only the polyfill's prototypes; nothing else.
import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';

const dir = process.env.BRADY_CAPTURE;
if (dir) {
  // Must patch the same webbluetooth instance the SDK shim imports: resolve from BRADY_CAPTURE_MODULES (the skill's scripts dir).
  const require = createRequire(path.join(path.resolve(process.env.BRADY_CAPTURE_MODULES ?? path.dirname(process.argv[1])), 'x.js'));
  const { BluetoothRemoteGATTCharacteristic: Char } = require('webbluetooth/dist/characteristic.js');
  const { BluetoothRemoteGATTServer: Server } = require('webbluetooth/dist/server.js');
  const { BluetoothRemoteGATTService: Service } = require('webbluetooth/dist/service.js');

  fs.mkdirSync(dir, { recursive: true });
  const stamp = new Date().toISOString().replace(/[:.]/g, '-');
  const label = process.env.BRADY_CAPTURE_LABEL ?? '';
  const file = path.join(dir, `${stamp}${label ? '-' + label : ''}.jsonl`);
  const t0 = performance.now();
  const log = (rec) => fs.appendFileSync(file, JSON.stringify({ t_ms: +(performance.now() - t0).toFixed(1), ...rec }) + '\n');
  const bytesOf = (v) => (v instanceof ArrayBuffer ? new Uint8Array(v) : new Uint8Array(v.buffer, v.byteOffset ?? 0, v.byteLength));
  const hex = (v) => Buffer.from(bytesOf(v)).toString('hex');
  const short = (uuid) => uuid.slice(0, 8);

  log({ ev: 'start', label, argv: process.argv.slice(1), file });
  process.on('exit', (code) => log({ ev: 'exit', code }));

  const pServerConnect = Server.prototype.connect;
  Server.prototype.connect = async function () {
    log({ ev: 'connect', device: this.device?.name, id: this.device?.id });
    try {
      const r = await pServerConnect.call(this);
      log({ ev: 'connected' });
      this.device?.addEventListener?.('gattserverdisconnected', () => log({ ev: 'disconnected-event' }));
      return r;
    } catch (e) { log({ ev: 'connect-error', err: e.message }); throw e; }
  };
  const pServerDisconnect = Server.prototype.disconnect;
  Server.prototype.disconnect = function () { log({ ev: 'disconnect' }); return pServerDisconnect.call(this); };

  const pServices = Server.prototype.getPrimaryServices;
  Server.prototype.getPrimaryServices = async function (s) {
    const r = await pServices.call(this, s);
    log({ ev: 'services', requested: s ?? null, uuids: r.map((x) => x.uuid) });
    return r;
  };
  const pChars = Service.prototype.getCharacteristics;
  Service.prototype.getCharacteristics = async function (c) {
    const r = await pChars.call(this, c);
    log({ ev: 'characteristics', service: this.uuid, requested: c ?? null, chars: r.map((x) => ({ uuid: x.uuid, props: Object.entries(x.properties).filter(([, v]) => v).map(([k]) => k) })) });
    return r;
  };

  const pWrite = Char.prototype.writeValue;
  Char.prototype.writeValue = async function (value, withoutResponse = false) {
    // The polyfill sends value.buffer whole, so record exactly that, plus the view bounds if they differ.
    const sent = value instanceof ArrayBuffer ? new Uint8Array(value) : new Uint8Array(value.buffer);
    const rec = { ev: 'tx', char: short(this.uuid), mode: withoutResponse ? 'cmd' : 'req', len: sent.length, hex: Buffer.from(sent).toString('hex') };
    if (!(value instanceof ArrayBuffer) && (value.byteOffset !== 0 || value.byteLength !== value.buffer.byteLength)) rec.view = [value.byteOffset, value.byteLength];
    log(rec);
    const t = performance.now();
    try {
      await pWrite.call(this, value, withoutResponse);
      log({ ev: 'tx-done', char: short(this.uuid), took_ms: +(performance.now() - t).toFixed(1) });
    } catch (e) { log({ ev: 'tx-error', char: short(this.uuid), err: e.message }); throw e; }
  };

  const pRead = Char.prototype.readValue;
  Char.prototype.readValue = async function () {
    this.__reading = true;
    try {
      const v = await pRead.call(this);
      log({ ev: 'rx-read', char: short(this.uuid), len: v.byteLength, hex: hex(v) });
      return v;
    } catch (e) { log({ ev: 'read-error', char: short(this.uuid), err: e.message }); throw e; }
    finally { this.__reading = false; }
  };

  const pSet = Char.prototype.setValue;
  Char.prototype.setValue = function (value, emit) {
    if (emit && value && !this.__reading) log({ ev: 'rx', char: short(this.uuid), len: value.byteLength, hex: hex(value) });
    return pSet.call(this, value, emit);
  };

  for (const m of ['startNotifications', 'stopNotifications']) {
    const p = Char.prototype[m];
    Char.prototype[m] = async function () { log({ ev: m, char: short(this.uuid) }); return p.call(this); };
  }
  process.stderr.write(`[capture] ${file}\n`);
}
