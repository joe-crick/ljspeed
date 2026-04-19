#!/usr/bin/env node
const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

// Path to the compiled Rust binary
const binaryPath = path.join(__dirname, '../target/release/ljsp-macro-compiler-rs');

if (!fs.existsSync(binaryPath)) {
  console.error('Error: LJSP Macro Compiler binary not found.');
  console.error('Please ensure you have Rust installed and run "cargo build --release" in the package directory.');
  process.exit(1);
}

const args = process.argv.slice(2);
const child = spawn(binaryPath, args, { stdio: 'inherit' });

child.on('exit', (code) => {
  process.exit(code || 0);
});

child.on('error', (err) => {
  console.error('Failed to start the compiler:', err);
  process.exit(1);
});
