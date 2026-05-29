#!/usr/bin/env node
'use strict';
const { spawnSync } = require('child_process');
const path = require('path');
const bin = path.join(__dirname, '..', 'dist', 'sim-test');
const r = spawnSync(bin, process.argv.slice(2), { stdio: 'inherit' });
process.exit(r.status ?? 1);
