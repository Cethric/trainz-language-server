#!/usr/bin/env node

const {execSync} = require('child_process');
const path = require('path');

const [, , newVersion] = process.argv;

if (!newVersion) {
    console.error('Usage: node update-all-crates.js <new_version>');
    process.exit(1);
}

const crates = [
    'ast',
    'common',
    'formatter',
    'formatter-cli',
    'diagnostics',
    'language-server',
    'parser',
    'semantic-tokens',
    'symboliser',
    'completions',
    'folding',
    'hover',
    'definition'
];

const scriptPath = path.join(__dirname, 'update-crate-version.js');
for (const crate of crates) {
    console.log(`Updating crate ${crate}...`);
    // Path to update-crate-version.js is in the same directory
    execSync(`node "${scriptPath}" ${crate} ${newVersion}`, {stdio: 'inherit'});
}
