#!/usr/bin/env node

const fs = require("fs");
const path = require("path");

const [outputPath, version] = process.argv.slice(2);

if (!outputPath || !version) {
    throw new Error("Usage: render-main-package.cjs <output-path> <version>");
}

// The platform binary packages the launcher resolves at runtime. Kept in lockstep with the
// release-npm.yml build matrix.
const platformSuffixes = [
    "darwin-arm64",
    "darwin-x64",
    "linux-arm64",
    "linux-x64",
    "windows-arm64",
    "windows-x64",
];

const optionalDependencies = {};
for (const suffix of platformSuffixes) {
    optionalDependencies[`@getsigit/si-${suffix}`] = version;
}

const packageJson = {
    name: "@getsigit/si",
    version,
    // Ownership proof for the MCP Registry: must match `name` in the repo's server.json.
    mcpName: "si.sigit/cli",
    keywords: ["sigit", "si", "cli", "git", "code-review", "mcp"],
    bin: {
        si: "lib/index.js",
    },
    files: ["lib", "README.md"],
    description: "si, the sigit.si command line interface.",
    license: "Apache-2.0",
    homepage: "https://sigit.si",
    repository: {
        type: "git",
        url: "git+https://github.com/getsigit/si.git",
    },
    scripts: {
        typecheck: "tsc --noEmit",
        build: "tsc",
        dev: "npm run build && node lib/index.js",
    },
    devDependencies: {
        "@types/node": "^18.15.11",
        typescript: "^5.0.0",
    },
    optionalDependencies,
};

fs.writeFileSync(path.resolve(outputPath), `${JSON.stringify(packageJson, null, 2)}\n`);
