#!/usr/bin/env node

import { spawn } from 'child_process';
import { fileURLToPath } from 'url';
import path from 'path';
import fs from 'fs';
import os from 'os';

// Get the directory of this script
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Detect platform and architecture
const platform = os.platform();
const arch = os.arch();

// Map Node.js platform/arch to our vendor directory structure
function getPlatformKey() {
  const platformMap = {
    'darwin-x64': 'darwin-x64',
    'darwin-arm64': 'darwin-arm64',
    'linux-x64': 'linux-x64',
    'linux-arm64': 'linux-arm64',
    'win32-x64': 'win32-x64',
  };

  const key = `${platform}-${arch}`;
  return platformMap[key];
}

// Find the binary path
function getBinaryPath() {
  const platformKey = getPlatformKey();

  if (!platformKey) {
    console.error(`Unsupported platform: ${platform}-${arch}`);
    console.error('Supported platforms: darwin-x64, darwin-arm64, linux-x64, linux-arm64, win32-x64');
    process.exit(1);
  }

  const binaryName = platform === 'win32' ? 'retrochat.exe' : 'retrochat';
  const binaryPath = path.join(__dirname, '..', 'vendor', platformKey, 'retrochat', binaryName);

  if (!fs.existsSync(binaryPath)) {
    console.error(`Binary not found at: ${binaryPath}`);
    console.error('This installation may be corrupted. Please try reinstalling:');
    console.error('  npm install -g @sanggggg/retrochat --force');
    process.exit(1);
  }

  return binaryPath;
}

// Main execution
const binaryPath = getBinaryPath();

// Spawn the binary with all arguments
const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
  windowsHide: false,
});

// Handle signals properly
const signals = ['SIGINT', 'SIGTERM', 'SIGHUP'];
signals.forEach((signal) => {
  process.on(signal, () => {
    if (!child.killed) {
      child.kill(signal);
    }
  });
});

// Mirror child process exit
child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
  } else {
    process.exit(code ?? 1);
  }
});

// Handle child process errors
child.on('error', (err) => {
  console.error('Failed to start retrochat:', err.message);
  process.exit(1);
});
