#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

/**
 * Update Cargo.toml version fields with the new semantic version
 * Usage: node update-cargo-versions.js <version>
 */

const version = process.argv[2];
if (!version) {
  console.error('Usage: node update-cargo-versions.js <version>');
  process.exit(1);
}

console.log(`Updating Cargo.toml files to version ${version}`);

// Update root Cargo.toml
const rootCargoPath = path.join(__dirname, 'Cargo.toml');
if (fs.existsSync(rootCargoPath)) {
  let content = fs.readFileSync(rootCargoPath, 'utf8');
  content = content.replace(/version = "[\d\.]+"\s*$/gm, `version = "${version}"`);
  fs.writeFileSync(rootCargoPath, content);
  console.log('Updated root Cargo.toml');
}

// Update all crate Cargo.toml files
const cratesDir = path.join(__dirname, 'crates');
if (fs.existsSync(cratesDir)) {
  const crates = fs.readdirSync(cratesDir);
  crates.forEach(crate => {
    const crateCargoPath = path.join(cratesDir, crate, 'Cargo.toml');
    if (fs.existsSync(crateCargoPath)) {
      let content = fs.readFileSync(crateCargoPath, 'utf8');
      content = content.replace(/version = "[\d\.]+"\s*$/gm, `version = "${version}"`);
      fs.writeFileSync(crateCargoPath, content);
      console.log(`Updated ${crate}/Cargo.toml`);
    }
  });
}

// Update workspace dependencies in root Cargo.toml
if (fs.existsSync(rootCargoPath)) {
  let content = fs.readFileSync(rootCargoPath, 'utf8');
  content = content.replace(/version = "[\d\.]+"\s*$/gm, `version = "${version}"`);
  fs.writeFileSync(rootCargoPath, content);
  console.log('Updated workspace dependency versions');
}

console.log('Cargo.toml version updates complete');