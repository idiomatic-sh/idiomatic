#!/usr/bin/env node
// Runs the native tastemaker binary from the platform package npm installed for this
// machine, one of this package's optionalDependencies, passing its arguments, standard
// streams, signals and exit status through.
"use strict";

const { spawn } = require("node:child_process");
const { optionalDependencies } = require("./package.json");

const executable = process.platform === "win32" ? "tm.exe" : "tm";

function binary() {
  for (const name of Object.keys(optionalDependencies ?? {})) {
    try {
      return require.resolve(`${name}/bin/${executable}`);
    } catch {
      // Not this machine's platform: npm skipped the package.
    }
  }
  throw new Error(
    `@idiomatic-sh/tastemaker: no platform package is installed for ${process.platform}-${process.arch}; reinstall without --omit=optional`,
  );
}

const forwarded = ["SIGINT", "SIGTERM", "SIGHUP"];
const child = spawn(binary(), process.argv.slice(2), { stdio: "inherit" });
for (const signal of forwarded) {
  // A signal sent to this process alone reaches the binary too.
  process.on(signal, () => child.kill(signal));
}
child.on("error", (error) => {
  throw error;
});
child.on("exit", (status, signal) => {
  if (signal) {
    // End the way the binary ended, so the caller sees the same signal.
    for (const name of forwarded) {
      process.removeAllListeners(name);
    }
    process.kill(process.pid, signal);
    return;
  }
  process.exit(status ?? 1);
});
