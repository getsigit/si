#!/usr/bin/env node

import { spawnSync } from "child_process";

/**
 * Resolves the native binary from the platform package npm installed next to this one.
 * The package is named `@getsigit/si-${os}-${arch}`, with `win32` mapped to `windows`.
 * @see https://nodejs.org/api/os.html#osarch
 * @see https://nodejs.org/api/os.html#osplatform
 */
function getExePath(): string {
    const arch = process.arch;
    let os = process.platform as string;
    let extension = "";
    if (["win32", "cygwin"].includes(process.platform)) {
        os = "windows";
        extension = ".exe";
    }

    try {
        return require.resolve(`@getsigit/si-${os}-${arch}/bin/si${extension}`);
    } catch (e) {
        throw new Error(
            `Couldn't find the si binary for ${os}-${arch} inside node_modules. ` +
                "Reinstall without --omit=optional, or install with `cargo install sigit-si-cli`.",
        );
    }
}

function run(): void {
    const args = process.argv.slice(2);
    const result = spawnSync(getExePath(), args, { stdio: "inherit" });

    if (result.error) {
        console.error(`Failed to run si: ${result.error.message}`);
        process.exit(1);
    }

    // `status` is null when the child was killed by a signal; report that as a failure rather
    // than as success.
    process.exit(result.status ?? 1);
}

run();
