#!/usr/bin/env node
// Capture driver: runs Brady's SDK (unmodified, from the brady-m511 skill's node_modules) through the
// polyfill hook and exercises only its public surface, the same calls server/printer.mjs makes.
//   BRADY_CAPTURE=captures/<session> node --import ./hook.mjs drive.mjs <cmd> [opts]
//   cmds: info | idle --seconds 60 | print --image x.png [--copies 1] [--cut 0|1|2] [--x -0.12] [--y 0] | feed | cut
// Saves the canvas the SDK rasterised (via our shim) next to the capture as <label>-raster.png.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { createRequire } from 'node:module';

const SKILL = path.join(os.homedir(), '.claude/skills/brady-m511/scripts');
const argv = process.argv.slice(2);
const cmd = argv[0];
const arg = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
if (!['info', 'idle', 'print', 'feed', 'cut'].includes(cmd)) {
  console.error('usage: drive.mjs info|idle|print|feed|cut [--image x.png] [--copies 1] [--cut 0] [--x -0.12] [--y 0] [--seconds 60] [--scan 60]');
  process.exit(1);
}
const { installShims, canvases } = await import(path.join(SKILL, 'shim.mjs'));
installShims({ scanSeconds: +arg('--scan', 60) });
const { Image } = createRequire(path.join(SKILL, 'x.js'))('canvas');
const { default: BradySdk } = await import(path.join(SKILL, 'node_modules/@bradycorporation/brady-web-sdk/dist/bundle.js'));
const origLog = console.log;
console.log = (...a) => { if (!String(a[0]).startsWith('PICL packet received')) origLog(...a); };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const t0 = performance.now();
const note = (text) => console.error(`[drive +${((performance.now() - t0) / 1000).toFixed(2)}s] ${text}`);

let updates = 0;
const sdk = new BradySdk(() => { updates++; }, false);
note('scanning');
let ok = false;
try { ok = await sdk.showDiscoveredBleDevices(); } catch (e) { note('connect error: ' + e.message); }
if (!ok || !sdk.isConnected()) { note('NOT CONNECTED'); process.exit(2); }
note('connected');
for (let i = 0; i < 60 && (sdk.supplyWidth == null || sdk.dotsPerInch == null); i++) await sleep(250);
const info = () => ({
  printer: sdk.printerName, model: sdk.printerModel, status: sdk.status, firmware: sdk.firmwareVersion,
  supplyName: sdk.supplyName, supplyYNumber: sdk.supplyYNumber, supplyWidthIn: sdk.supplyWidth, supplyHeightIn: sdk.supplyHeight,
  dieCut: sdk.mediaIsDieCut, dpi: sdk.dotsPerInch, zone: sdk.zoneDimensions,
  supplyRemainingPct: sdk.supplyRemainingPercentage, batteryPct: sdk.batteryLevelPercentage, acConnected: sdk.isAcConnected,
  message: sdk.message, messageTitle: sdk.messageTitle, errorSeverity: sdk.errorSeverity, updates,
});
note('info ' + JSON.stringify(info()));

if (cmd === 'idle') {
  const secs = +arg('--seconds', 60);
  note(`idle for ${secs}s, logging SDK-visible state every 5s`);
  for (let i = 0; i < secs; i += 5) { await sleep(5000); note('state ' + JSON.stringify(info())); if (!sdk.isConnected()) { note('sdk reports disconnected'); break; } }
} else if (cmd === 'print') {
  const imagePath = arg('--image');
  const png = fs.readFileSync(imagePath);
  const img = new Image();
  await new Promise((res, rej) => { img.onload = res; img.onerror = rej; img.src = 'data:image/png;base64,' + png.toString('base64'); });
  const copies = +arg('--copies', 1), cut = +arg('--cut', 0), x = +arg('--x', -0.12), y = +arg('--y', 0);
  sdk.setCopies(copies);
  sdk.setCutOption(cut);
  note(`print ${imagePath} ${img.naturalWidth}x${img.naturalHeight} copies=${copies} cut=${cut} x=${x} y=${y}`);
  let res;
  try { res = await sdk.printBitmap(img, x, y); } catch (e) { note('print error: ' + e.message); }
  note(`print result ${res} | printer says: ${sdk.messageTitle ?? ''} ${sdk.message ?? ''}`);
  const raster = canvases.find((c) => c.width > 1);
  if (raster) {
    const out = path.join(process.env.BRADY_CAPTURE ?? '.', `${process.env.BRADY_CAPTURE_LABEL ?? path.basename(imagePath, '.png')}-raster.png`);
    fs.writeFileSync(out, raster.toBuffer('image/png'));
    note(`raster ${raster.width}x${raster.height} -> ${out}`);
  }
  await sleep(+arg('--after', 4000));
  note('after-print ' + JSON.stringify(info()));
} else if (cmd === 'feed') {
  note('feed'); note(`feed result ${await sdk.feed()}`); await sleep(3000);
} else if (cmd === 'cut') {
  note('cut'); note(`cut result ${await sdk.cut()}`); await sleep(3000);
}
note('disconnect');
try { await sdk.disconnect(); } catch (e) { note('disconnect error: ' + e.message); }
await sleep(500);
process.exit(0);
