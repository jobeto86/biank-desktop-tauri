import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import os from 'node:os';

// Only probes the native API. Never changes clock, power state, registry or locks.
const [binaryPath, receiptPath] = process.argv.slice(2);
assert.equal(process.argv.length, 4, 'Usage: node validate.mjs <binary> <receipt.json>');
assert.equal(process.platform, 'win32', 'Native Windows is required; Linux is not evidence');
const binary = resolve(binaryPath);
const snapshots = [];
for (let i = 0; i < 12; i++) {
  const result = spawnSync(binary, [], { encoding: 'utf8', timeout: 2000, maxBuffer: 16384, windowsHide: true });
  assert.ifError(result.error);
  assert.equal(result.status, 0, result.stderr);
  const snapshot = JSON.parse(result.stdout);
  assert.equal(snapshot.schemaVersion, 1);
  assert.equal(snapshot.platform, 'windows');
  assert.equal(snapshot.algorithm, 'nt-boot-environment-v1');
  assert.equal(snapshot.status, 'observed', JSON.stringify(snapshot));
  assert.match(snapshot.bootId, /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/);
  assert.equal(snapshot.returnedBytes, 32);
  assert.equal(snapshot.bootFlagsOffset, 24);
  snapshots.push(snapshot);
}
assert.equal(new Set(snapshots.map(s => s.bootId)).size, 1, 'Fresh processes disagree about the boot');
const receipt = {
  schemaVersion: 1,
  sourceSha: process.env.GITHUB_SHA || null,
  binarySha256: createHash('sha256').update(readFileSync(binary)).digest('hex'),
  osRelease: os.release(),
  osVersion: os.version(),
  runnerImage: process.env.ImageOS || null,
  runnerImageVersion: process.env.ImageVersion || null,
  collectedAt: new Date().toISOString(),
  verdict: 'API_AND_SAME_BOOT_PASS_ONLY',
  permissions: 'current-token; no elevation requested by probe',
  pending: ['standard-user', 'real-reboot', 'fast-startup', 'suspend-resume', 'hibernate-resume', 'clock-adjustment'],
  snapshots,
};
writeFileSync(receiptPath, JSON.stringify(receipt, null, 2) + '\n', { flag: 'wx' });
console.log(JSON.stringify({ verdict: receipt.verdict, osRelease: receipt.osRelease, snapshots: snapshots.length, pending: receipt.pending }));
