'use strict';
const https = require('https');
const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

const DIST_DIR = path.join(__dirname, '..', 'dist');
const VERSION = require('../package.json').version;
const REPO = 'BigBangStudios/sim-mcp';

/** Detect the platform string used in release asset names. */
function detectPlatform() {
  const os = process.platform;
  const arch = process.arch;
  if (os === 'darwin' && arch === 'arm64') return 'darwin-arm64';
  if (os === 'darwin' && arch === 'x64')  return 'darwin-x64';
  if (os === 'linux'  && arch === 'x64')  return 'linux-x64';
  throw new Error(`Unsupported platform: ${os}-${arch}`);
}

/** Map platform string to the Rust target triple used in release asset names. */
function platformToTarget(platform) {
  const map = {
    'darwin-arm64': 'aarch64-apple-darwin',
    'darwin-x64':   'x86_64-apple-darwin',
    'linux-x64':    'x86_64-unknown-linux-gnu',
  };
  if (!map[platform]) throw new Error(`Unknown platform: ${platform}`);
  return map[platform];
}

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    const get = (u, depth = 0) => {
      if (depth > 5) {
        reject(new Error(`Too many redirects for ${url}`));
        return;
      }
      https.get(u, (res) => {
        if (res.statusCode === 301 || res.statusCode === 302) {
          get(res.headers.location, depth + 1);
          return;
        }
        if (res.statusCode !== 200) {
          reject(new Error(`HTTP ${res.statusCode} for ${u}`));
          return;
        }
        res.pipe(file);
        file.on('finish', () => file.close(resolve));
      }).on('error', reject);
    };
    get(url);
  });
}

async function install() {
  // Allow skipping in environments that don't need the binary.
  if (process.env.SIM_SKIP_INSTALL) return;

  const platform = detectPlatform();
  const target = platformToTarget(platform);

  fs.mkdirSync(DIST_DIR, { recursive: true });

  const binaries = ['sim', 'sim-mcp', 'sim-test'];
  for (const name of binaries) {
    const asset = `${name}-v${VERSION}-${target}.tar.gz`;
    const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${asset}`;
    const tarPath = path.join(DIST_DIR, asset);

    process.stdout.write(`sim-mcp: downloading ${name}... `);
    await download(url, tarPath);

    try {
      execFileSync('tar', ['-xzf', tarPath, '-C', DIST_DIR]);
    } finally {
      try { fs.unlinkSync(tarPath); } catch (_) {}
    }

    const binPath = path.join(DIST_DIR, name);
    fs.chmodSync(binPath, 0o755);
    process.stdout.write('done\n');
  }
}

module.exports = { detectPlatform, platformToTarget };

if (require.main === module) {
  install().catch((err) => {
    console.error('sim-mcp install failed:', err.message);
    process.exit(1);
  });
}
