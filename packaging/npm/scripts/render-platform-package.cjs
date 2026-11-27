#!/usr/bin/env node

const fs = require("fs");
const path = require("path");

const [packageName, version, operatingSystem, architecture] = process.argv.slice(2);

if (!packageName || !version || !operatingSystem || !architecture) {
    throw new Error("Usage: render-platform-package.cjs <package-name> <version> <os> <arch>");
}

const packageDirectory = path.resolve(__dirname, "..", packageName);

fs.mkdirSync(packageDirectory, { recursive: true });

const packageJson = {
    name: `@getsigit/${packageName}`,
    version,
    description: "Platform binary for the si CLI.",
    license: "Apache-2.0",
    repository: {
        type: "git",
        url: "git+https://github.com/getsigit/si.git",
    },
    keywords: [packageName, "si", "sigit", "cli", "binary"],
    os: [operatingSystem],
    cpu: [architecture],
    files: ["bin"],
};

const readme = `# ${packageName}

Not meant to be used directly. Please see [@getsigit/si](https://www.npmjs.com/package/@getsigit/si).
`;

fs.writeFileSync(path.join(packageDirectory, "package.json"), `${JSON.stringify(packageJson, null, 2)}\n`);
fs.writeFileSync(path.join(packageDirectory, "README.md"), readme);
