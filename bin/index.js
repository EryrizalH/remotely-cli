#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

const binaryName = process.platform === 'win32' ? 'remotely.exe' : 'remotely';
const binaryPath = path.join(__dirname, binaryName);

if (!fs.existsSync(binaryPath)) {
  console.error(`Error: remotely binary not found at ${binaryPath}`);
  console.error('Please try reinstalling this package: npm install -g remotely-cli');
  process.exit(1);
}

// Spawn the native binary and forward all arguments, stdin, stdout, stderr
const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
  env: process.env
});

child.on('error', (err) => {
  console.error(`Failed to start native binary: ${err.message}`);
  process.exit(1);
});

child.on('exit', (code, signal) => {
  if (code !== null) {
    process.exit(code);
  } else if (signal) {
    process.kill(process.pid, signal);
  } else {
    process.exit(0);
  }
});
