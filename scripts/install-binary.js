const fs = require('fs');
const path = require('path');
const https = require('https');
const { spawnSync } = require('child_process');
const os = require('os');

const OWNER = 'joe-crick';
const REPO = 'ljspeed';
const pkg = JSON.parse(fs.readFileSync(path.join(__dirname, '../package.json'), 'utf8'));
const VERSION = `v${pkg.version}`;

const platformMap = {
  'linux-x64': `ljspeed-linux-x64`,
  'darwin-x64': `ljspeed-macos-x64`,
  'darwin-arm64': `ljspeed-macos-arm64`,
  'win32-x64': `ljspeed-windows-x64.exe`
};

const platform = `${os.platform()}-${os.arch()}`;
const binaryName = platformMap[platform];
const binDir = path.join(__dirname, '../bin');
const targetPath = path.join(binDir, os.platform() === 'win32' ? 'ljspeed.exe' : 'ljspeed');

function buildFromSource() {
  console.log('Falling back to building from source...');
  const result = spawnSync('cargo', ['build', '--release'], { stdio: 'inherit', shell: true });
  if (result.status !== 0) {
    console.error('Error: Failed to build from source. Please ensure Rust/Cargo is installed.');
    process.exit(1);
  }
  
  const compiledPath = path.join(__dirname, '../target/release', os.platform() === 'win32' ? 'ljspeed.exe' : 'ljspeed');
  if (fs.existsSync(compiledPath)) {
    if (!fs.existsSync(binDir)) fs.mkdirSync(binDir);
    fs.copyFileSync(compiledPath, targetPath);
    fs.chmodSync(targetPath, 0o755);
  }
}

if (!binaryName) {
  console.log(`Precompiled binary not available for platform: ${platform}`);
  buildFromSource();
  process.exit(0);
}

const url = `https://github.com/${OWNER}/${REPO}/releases/download/${VERSION}/${binaryName}`;

console.log(`Downloading ljspeed from ${url}...`);

if (!fs.existsSync(binDir)) fs.mkdirSync(binDir);

const file = fs.createWriteStream(targetPath);
https.get(url, (response) => {
  if (response.statusCode === 302 || response.statusCode === 301) {
    https.get(response.headers.location, (res) => {
      res.pipe(file);
      file.on('finish', () => {
        file.close();
        fs.chmodSync(targetPath, 0o755);
        console.log('Successfully installed ljspeed.');
      });
    });
  } else if (response.statusCode === 200) {
    response.pipe(file);
    file.on('finish', () => {
      file.close();
      fs.chmodSync(targetPath, 0o755);
      console.log('Successfully installed ljspeed.');
    });
  } else {
    console.warn(`Warning: Download failed with status ${response.statusCode}.`);
    file.close();
    fs.unlinkSync(targetPath);
    buildFromSource();
  }
}).on('error', (err) => {
  console.warn(`Warning: Download error: ${err.message}`);
  file.close();
  if (fs.existsSync(targetPath)) fs.unlinkSync(targetPath);
  buildFromSource();
});
