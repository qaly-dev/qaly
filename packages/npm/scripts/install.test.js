// Tests for pure helper functions only — no network calls.
'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { detectPlatform, platformToTarget } = require('./install');

test('detectPlatform returns known value', () => {
  const p = detectPlatform();
  assert.ok(['darwin-arm64', 'darwin-x64', 'linux-x64'].includes(p));
});

test('detectPlatform returns string', () => {
  assert.equal(typeof detectPlatform(), 'string');
});

test('platformToTarget maps darwin-arm64', () => {
  assert.equal(platformToTarget('darwin-arm64'), 'aarch64-apple-darwin');
});

test('platformToTarget maps linux-x64', () => {
  assert.equal(platformToTarget('linux-x64'), 'x86_64-unknown-linux-gnu');
});

test('platformToTarget throws for unknown platform', () => {
  assert.throws(() => platformToTarget('windows-x64'), /Unknown platform/);
});
