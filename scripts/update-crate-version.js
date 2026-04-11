#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

/**
 * Update a specific crate's version and its workspace dependency version
 * Usage: node update-crate-version.js <crate_name> <new_version>
 */

const [, , crateName, newVersion] = process.argv;

if (!crateName || !newVersion) {
    console.error('Usage: node update-crate-version.js <crate_name> <new_version>');
    process.exit(1);
}

console.log(`Updating crate ${crateName} to version ${newVersion}`);

// 1. Update crate Cargo.toml
const crateCargoPath = path.join(__dirname, '..', 'crates', crateName, 'Cargo.toml');
if (fs.existsSync(crateCargoPath)) {
    let content = fs.readFileSync(crateCargoPath, 'utf8');
    // Be more specific: only replace version in [package] section
    // Replace version = "x.y.z"
    content = content.replace(/^version = "[^"]+"/m, `version = "${newVersion}"`);
    fs.writeFileSync(crateCargoPath, content);
    console.log(`Updated crates/${crateName}/Cargo.toml`);
} else {
    console.error(`Crate Cargo.toml not found at ${crateCargoPath}`);
    process.exit(1);
}

// 2. Update workspace dependency in root Cargo.toml
const rootCargoPath = path.join(__dirname, '..', 'Cargo.toml');
if (fs.existsSync(rootCargoPath)) {
    let content = fs.readFileSync(rootCargoPath, 'utf8');
    // Find the line with 'trainz-<crateName> = { path = "crates/<crateName>", version = "..." }'
    const dependencyKey = `trainz-${crateName}`;
    // Regex to match the version inside the dependency definition
    const regex = new RegExp(`(${dependencyKey}\\s*=\\s*\\{.*version\\s*=\\s*")([^"]+)"`, 'g');

    if (regex.test(content)) {
        content = content.replace(regex, `$1${newVersion}"`);
        fs.writeFileSync(rootCargoPath, content);
        console.log(`Updated workspace dependency ${dependencyKey} to ${newVersion} in Cargo.toml`);
    } else {
        console.warn(`Could not find dependency ${dependencyKey} in root Cargo.toml`);
    }
}
