const fs = require('fs');
const path = require('path');
const https = require('https');
const http = require('http');
const { exec } = require('child_process');

// Configuration
const REPO = 'EryrizalH/teleprompt-cli';
const packageJson = require('../package.json');
const VERSION = packageJson.version;

const platform = process.platform;
const arch = process.arch;

const binDir = path.join(__dirname, '..', 'bin');
const binaryName = platform === 'win32' ? 'teleprompt.exe' : 'teleprompt';
const binaryPath = path.join(binDir, binaryName);

// Map OS & Arch to Release Asset filenames
const assets = {
  'win32-x64': {
    name: 'teleprompt-windows-x64.zip',
    type: 'zip'
  },
  'darwin-x64': {
    name: 'teleprompt-macos-x64.tar.gz',
    type: 'tar'
  },
  'darwin-arm64': {
    name: 'teleprompt-macos-arm64.tar.gz',
    type: 'tar'
  },
  'linux-x64': {
    name: 'teleprompt-linux-x64.tar.gz',
    type: 'tar'
  }
};

const key = `${platform}-${arch}`;
const assetInfo = assets[key];

async function main() {
  // Ensure bin directory exists
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }

  // If binary already exists (e.g. from local compilation or dev setup), skip download
  if (fs.existsSync(binaryPath)) {
    console.log(`Binary already exists at ${binaryPath}. Skipping download.`);
    // Make sure it is executable on Unix
    if (platform !== 'win32') {
      try {
        fs.chmodSync(binaryPath, 0o755);
      } catch (err) {
        // Ignore chmod error if it fails
      }
    }
    return;
  }

  if (!assetInfo) {
    console.log(`No precompiled binary available for platform: ${platform}, arch: ${arch}`);
    await tryBuildFromSource();
    return;
  }

  const downloadUrl = `https://github.com/${REPO}/releases/download/v${VERSION}/${assetInfo.name}`;
  const archivePath = path.join(binDir, assetInfo.name);

  console.log(`Downloading precompiled binary from: ${downloadUrl}`);
  try {
    await downloadFile(downloadUrl, archivePath);
    console.log(`Successfully downloaded archive to ${archivePath}`);

    console.log(`Extracting archive...`);
    await extractArchive(archivePath, binDir, assetInfo.type);
    
    // Clean up archive file
    if (fs.existsSync(archivePath)) {
      fs.unlinkSync(archivePath);
    }

    // Set execution permissions on Unix
    if (platform !== 'win32') {
      console.log(`Setting executable permissions on binary...`);
      fs.chmodSync(binaryPath, 0o755);
    }

    console.log(`teleprompt-cli installed successfully!`);
  } catch (err) {
    console.error(`Failed to download or install precompiled binary: ${err.message}`);
    // Clean up archive if download failed halfway
    if (fs.existsSync(archivePath)) {
      fs.unlinkSync(archivePath);
    }
    await tryBuildFromSource();
  }
}

function downloadFile(url, destPath) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(destPath);
    
    function get(requestUrl) {
      const client = requestUrl.startsWith('https') ? https : http;
      client.get(requestUrl, (response) => {
        // Follow redirect status codes (301, 302, 307, 308)
        if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
          get(response.headers.location);
          return;
        }
        
        if (response.statusCode !== 200) {
          reject(new Error(`Server responded with status ${response.statusCode}`));
          return;
        }
        
        response.pipe(file);
        
        file.on('finish', () => {
          file.close(resolve);
        });
      }).on('error', (err) => {
        fs.unlink(destPath, () => {});
        reject(err);
      });
    }
    
    get(url);
  });
}

function extractArchive(archivePath, targetDir, archiveType) {
  return new Promise((resolve, reject) => {
    let cmd;
    if (platform === 'win32') {
      // Windows 10/11 includes a native tar utility in System32
      cmd = `tar -xf "${archivePath}" -C "${targetDir}"`;
    } else {
      cmd = `tar -xzf "${archivePath}" -C "${targetDir}"`;
    }
    
    exec(cmd, (error, stdout, stderr) => {
      if (error) {
        reject(new Error(`Extraction command failed: ${stderr || error.message}`));
      } else {
        resolve();
      }
    });
  });
}

function tryBuildFromSource() {
  return new Promise((resolve, reject) => {
    console.log('Checking for Cargo to build from source...');
    exec('cargo --version', (err, stdout, stderr) => {
      if (err) {
        console.error('Cargo is not installed or not in PATH. Cannot compile from source.');
        process.exit(1);
      }
      
      console.log(`Cargo detected: ${stdout.trim()}`);
      console.log('Compiling teleprompt-cli from source...');
      
      exec('cargo build --release', { cwd: path.join(__dirname, '..') }, (buildErr, buildStdout, buildStderr) => {
        if (buildErr) {
          console.error(`Compilation failed: ${buildStderr || buildErr.message}`);
          process.exit(1);
        }
        
        const sourceBinary = platform === 'win32' 
          ? path.join(__dirname, '..', 'target', 'release', 'teleprompt.exe')
          : path.join(__dirname, '..', 'target', 'release', 'teleprompt');
          
        if (!fs.existsSync(sourceBinary)) {
          console.error(`Built binary not found at expected path: ${sourceBinary}`);
          process.exit(1);
        }
        
        try {
          fs.copyFileSync(sourceBinary, binaryPath);
          if (platform !== 'win32') {
            fs.chmodSync(binaryPath, 0o755);
          }
          console.log(`Compilation complete. Binary successfully built and placed at ${binaryPath}`);
          resolve();
        } catch (copyErr) {
          console.error(`Failed to copy compiled binary: ${copyErr.message}`);
          process.exit(1);
        }
      });
    });
  });
}

main().catch((err) => {
  console.error(`Installation failed: ${err.message}`);
  process.exit(1);
});
