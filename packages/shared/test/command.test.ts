import assert from "node:assert/strict";
import test from "node:test";

import {
  buildCliArgv,
  CommandValidationError,
  OPERATIONS,
} from "../src/command.js";

test("maps start to explicit non-interactive CLI arguments", () => {
  assert.deepEqual(
    buildCliArgv("start", {
      projectPath: "/tmp/demo/dist",
      appid: "wx123",
      autoPort: 9421,
      proxy: true,
      https: true,
    }),
    [
      "--json",
      "start",
      "--appid",
      "wx123",
      "--auto-port",
      "9421",
      "--proxy",
      "--https",
      "--",
      "/tmp/demo/dist",
    ],
  );
});

test("keeps CLI values after the argument separator", () => {
  assert.deepEqual(
    buildCliArgv("run", { js: "--not-a-cli-flag $(touch /tmp/nope)" }),
    ["--json", "run", "--", "--not-a-cli-flag $(touch /tmp/nope)"],
  );
});

test("serializes structured wx arguments as JSON strings", () => {
  assert.deepEqual(
    buildCliArgv("wx", {
      method: "setStorageSync",
      args: ["key", 123, true, { nested: null }],
    }),
    [
      "--json",
      "wx",
      "--",
      "setStorageSync",
      "\"key\"",
      "123",
      "true",
      "{\"nested\":null}",
    ],
  );
});

test("rejects implicit project selection and unsupported daemon operations", () => {
  assert.throws(
    () => buildCliArgv("start", {}),
    (error: unknown) =>
      error instanceof CommandValidationError && /projectPath/.test(error.message),
  );
  assert.throws(
    () => buildCliArgv("daemon.stop", {}),
    (error: unknown) =>
      error instanceof CommandValidationError && /unsupported operation/.test(error.message),
  );
});

test("exposes all supported non-daemon CLI operations", () => {
  assert.equal((OPERATIONS as readonly string[]).includes("daemon.stop"), false);
  assert.equal(OPERATIONS.includes("net.install-ca"), true);
  assert.equal(new Set(OPERATIONS).size, OPERATIONS.length);
});
