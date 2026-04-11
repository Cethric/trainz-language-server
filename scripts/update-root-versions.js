#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

const [, , newVersion] = process.argv;

if (!newVersion) {
    console.error('Usage: node update-root-versions.js <new_version>');
    process.exit(1);
}

console.log(`Updating root versions to ${newVersion}`);

// 1. Update root package.json
const rootPackageJsonPath = path.join(__dirname, '..', 'package.json');
if (fs.existsSync(rootPackageJsonPath)) {
    let content = fs.readFileSync(rootPackageJsonPath, 'utf8');
    content = content.replace(/"version": "[^"]+"/, `"version": "${newVersion}"`);
    fs.writeFileSync(rootPackageJsonPath, content);
    console.log(`Updated root package.json`);
}

// 2. Update root Cargo.toml workspace package version
const rootCargoPath = path.join(__dirname, '..', 'Cargo.toml');
if (fs.existsSync(rootCargoPath)) {
    let content = fs.readFileSync(rootCargoPath, 'utf8');
    // Use a more robust regex for Cargo.toml that can handle edition in between
    content = content.replace(/(\[workspace\.package\](?:.*\n)*?version = ")[^"]+"/, `$1${newVersion}"`);
    fs.writeFileSync(rootCargoPath, content);
    console.log(`Updated root Cargo.toml [workspace.package] version`);
}
