'use strict';
const https = require('https');
const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

const DIST_DIR = path.join(__dirname, '..', 'dist');
const VERSION = require('../package.json').version;
const REPO = 'qaly-dev/qaly';

const PLATFORMS = {
  'darwin-arm64': { target: 'aarch64-apple-darwin',     ext: '.tar.xz', exe: '' },
  'darwin-x64':   { target: 'x86_64-apple-darwin',      ext: '.tar.xz', exe: '' },
  'linux-x64':    { target: 'x86_64-unknown-linux-gnu', ext: '.tar.xz', exe: '' },
  'windows-x64':  { target: 'x86_64-pc-windows-msvc',   ext: '.zip',    exe: '.exe' },
};

function detectPlatform() {
  const os = process.platform;
  const arch = process.arch;
  if (os === 'darwin' && arch === 'arm64') return 'darwin-arm64';
  if (os === 'darwin' && arch === 'x64')   return 'darwin-x64';
  if (os === 'linux'  && arch === 'x64')   return 'linux-x64';
  if (os === 'win32'  && arch === 'x64')   return 'windows-x64';
  throw new Error(`Unsupported platform: ${os}-${arch}. Supported: macOS (arm64/x64), Linux (x64), Windows (x64).`);
}

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    const get = (u, depth = 0) => {
      if (depth > 5) { reject(new Error('Too many redirects')); return; }
      https.get(u, (res) => {
        if (res.statusCode === 301 || res.statusCode === 302) {
          get(res.headers.location, depth + 1);
          return;
        }
        if (res.statusCode !== 200) {
          reject(new Error(`HTTP ${res.statusCode} downloading ${u}`));
          return;
        }
        res.pipe(file);
        file.on('finish', () => file.close(resolve));
      }).on('error', reject);
    };
    get(url);
  });
}

function extract(archivePath, destDir, ext) {
  if (ext === '.zip') {
    execFileSync('powershell', [
      '-NoProfile', '-NonInteractive', '-Command',
      `Expand-Archive -Path '${archivePath}' -DestinationPath '${destDir}' -Force`,
    ]);
  } else {
    execFileSync('tar', ['-xJf', archivePath, '-C', destDir]);
  }
}

async function install() {
  if (process.env.QALY_SKIP_INSTALL) return;

  const key = detectPlatform();
  const { target, ext, exe } = PLATFORMS[key];

  fs.mkdirSync(DIST_DIR, { recursive: true });

  for (const name of ['qaly', 'qaly-mcp', 'qaly-test']) {
    const asset = `${name}-${target}${ext}`;
    const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${asset}`;
    const archivePath = path.join(DIST_DIR, asset);

    process.stdout.write(`qaly: downloading ${name}... `);
    await download(url, archivePath);

    try {
      extract(archivePath, DIST_DIR, ext);
    } finally {
      try { fs.unlinkSync(archivePath); } catch (_) {}
    }

    const binPath = path.join(DIST_DIR, name + exe);
    if (!exe) fs.chmodSync(binPath, 0o755);
    process.stdout.write('done\n');
  }
}

module.exports = { detectPlatform };

if (require.main === module) {
  install().catch((err) => {
    console.error('qaly install failed:', err.message);
    process.exit(1);
  });
}
