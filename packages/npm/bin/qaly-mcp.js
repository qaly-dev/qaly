#!/usr/bin/env node
'use strict';
const { spawnSync } = require('child_process');
const path = require('path');
const exe = process.platform === 'win32' ? '.exe' : '';
const bin = path.join(__dirname, '..', 'dist', 'qaly-mcp' + exe);
const r = spawnSync(bin, process.argv.slice(2), { stdio: 'inherit' });
process.exit(r.status ?? 1);
