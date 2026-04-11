#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

/**
 * Update a specific extension's version
 * Usage: node update-extension-version.js <extension_name> <new_version>
 * <extension_name> can be 'vscode' or 'trainz-idea'
 */

const [, , extName, newVersion] = process.argv;

if (!extName || !newVersion) {
    console.error('Usage: node update-extension-version.js <extension_name> <new_version>');
    process.exit(1);
}

console.log(`Updating extension ${extName} to version ${newVersion}`);

if (extName === 'vscode') {
    const packageJsonPath = path.join(__dirname, '..', 'extensions', 'vscode', 'package.json');
    if (fs.existsSync(packageJsonPath)) {
        let content = fs.readFileSync(packageJsonPath, 'utf8');
        // Replace "version": "x.y.z"
        content = content.replace(/"version": "[\d\.\-SNAPSHOT]+"/, `"version": "${newVersion}"`);
        fs.writeFileSync(packageJsonPath, content);
        console.log(`Updated extensions/vscode/package.json`);
    } else {
        console.error(`package.json not found at ${packageJsonPath}`);
        process.exit(1);
    }
} else if (extName === 'trainz-idea') {
    const gradleKtsPath = path.join(__dirname, '..', 'extensions', 'trainz-idea', 'build.gradle.kts');
    if (fs.existsSync(gradleKtsPath)) {
        let content = fs.readFileSync(gradleKtsPath, 'utf8');
        // Replace version = "..."
        content = content.replace(/version = "[\d\.\-SNAPSHOT]+"/, `version = "${newVersion}"`);
        fs.writeFileSync(gradleKtsPath, content);
        console.log(`Updated extensions/trainz-idea/build.gradle.kts`);
    } else {
        console.error(`build.gradle.kts not found at ${gradleKtsPath}`);
        process.exit(1);
    }
} else {
    console.error(`Unknown extension: ${extName}. Use 'vscode' or 'trainz-idea'.`);
    process.exit(1);
}
