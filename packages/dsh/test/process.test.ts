import assert from "node:assert/strict";
import test from "node:test";

import { buildCliArgv } from "@wxpilot/shared";

import {
  createOperationQueue,
  runWxpilot,
  STDERR_MAX_BYTES,
  STDOUT_MAX_BYTES,
  TERMINATE_GRACE_MS,
  WxpilotToolError,
  type CollectedRead,
  type WxpilotHandle,
  type WxpilotOutcome,
  type WxpilotProcessDeps,
  type WxpilotSpawnSpec,
} from "../src/process.js";

interface FakeRun {
  outcome: WxpilotOutcome;
  stdout: string;
  stderr?: string;
  stdoutLossy?: boolean;
  stdoutSpillPath?: string;
}

function makeHandle(run: FakeRun): WxpilotHandle {
  const read = (
    text: string,
    lossy = false,
    spillPath?: string,
  ): { readFrom(offset: number): CollectedRead } => ({
    readFrom: () => ({ text, lossy, spillPath }),
  });
  return {
    done: Promise.resolve(run.outcome),
    collected: {
      stdout: read(run.stdout, run.stdoutLossy ?? false, run.stdoutSpillPath),
      stderr: read(run.stderr ?? ""),
    },
  };
}

function makeDeps(
  runs: FakeRun[],
  spawned: WxpilotSpawnSpec[],
): WxpilotProcessDeps {
  let index = 0;
  return {
    async resolveExecutable() {
      return "/fake/bin/wxpilot";
    },
    spawn(spec) {
      spawned.push(spec);
      const run = runs[index];
      index += 1;
      if (run === undefined) {
        throw new Error("unexpected extra spawn");
      }
      return makeHandle(run);
    },
  };
}

function baseOptions(overrides: Partial<Parameters<typeof runWxpilot>[1]> = {}) {
  return {
    binary: "wxpilot",
    argv: ["--json", "view"],
    cwd: "/tmp/project",
    signal: new AbortController().signal,
    maxResultChars: 16_000,
    ...overrides,
  };
}

test("spawns the resolved binary and passes argv through verbatim", async () => {
  const spawned: WxpilotSpawnSpec[] = [];
  const signal = new AbortController().signal;
  const deps = makeDeps(
    [{ outcome: { exitCode: 0, signal: null }, stdout: '{"ok":true,"elements":[]}' }],
    spawned,
  );

  await runWxpilot(
    deps,
    baseOptions({ argv: ["--json", "tap", "--", "%3"], signal }),
  );

  assert.equal(spawned.length, 1);
  assert.deepEqual(spawned[0].argv, [
    "/fake/bin/wxpilot",
    "--json",
    "tap",
    "--",
    "%3",
  ]);
  assert.equal(spawned[0].cwd, "/tmp/project");
  assert.equal(spawned[0].signal, signal);
  assert.equal(spawned[0].graceMs, TERMINATE_GRACE_MS);
  assert.equal(spawned[0].stdio.stdin, "ignore");
  assert.deepEqual(spawned[0].stdio.stdout, { maxBytes: STDOUT_MAX_BYTES });
  assert.deepEqual(spawned[0].stdio.stderr, { maxBytes: STDERR_MAX_BYTES });
});

test("never double-injects --json for argv built by buildCliArgv", async () => {
  // Regression guard: `buildCliArgv` already emits a leading `--json`, and the
  // CLI rejects a repeated flag ("cannot be used multiple times"), so the
  // spawn path must forward the argv verbatim without prepending it again.
  const spawned: WxpilotSpawnSpec[] = [];
  const deps = makeDeps(
    [{ outcome: { exitCode: 0, signal: null }, stdout: '{"ok":true}' }],
    spawned,
  );

  await runWxpilot(
    deps,
    baseOptions({ argv: buildCliArgv("view", { compact: false }) }),
  );

  assert.equal(
    spawned[0].argv.filter((argument) => argument === "--json").length,
    1,
    "--json must appear exactly once",
  );
  assert.deepEqual(spawned[0].argv, [
    "/fake/bin/wxpilot",
    "--json",
    "view",
    "--compact=false",
  ]);
});

test("returns the parsed payload for a successful JSON run", async () => {
  const deps = makeDeps([
    { outcome: { exitCode: 0, signal: null }, stdout: '{"ok":true,"page":"pages/index/index"}' },
  ], []);

  const result = await runWxpilot(deps, baseOptions());

  assert.equal(result.ok, true);
  assert.equal(result.exitCode, 0);
  assert.deepEqual(result.data, { ok: true, page: "pages/index/index" });
  assert.match(result.text, /"page": "pages\/index\/index"/);
});

test("keeps a CLI-reported failure as a canonical result, not a throw", async () => {
  const deps = makeDeps([
    { outcome: { exitCode: 1, signal: null }, stdout: '{"ok":false,"error":"ref %9 expired"}', stderr: "hint: re-run view" },
  ], []);

  const result = await runWxpilot(deps, baseOptions());

  assert.equal(result.ok, false);
  assert.equal(result.exitCode, 1);
  assert.equal(result.data?.error, "ref %9 expired");
  assert.match(result.text, /stderr:\nhint: re-run view/);
});

test("falls back to raw text when stdout is not JSON", async () => {
  const deps = makeDeps([
    { outcome: { exitCode: 0, signal: null }, stdout: "wxpilot 0.2.0\n" },
  ], []);

  const result = await runWxpilot(deps, baseOptions());

  assert.equal(result.ok, false);
  assert.equal(result.data, null);
  assert.equal(result.text, "wxpilot 0.2.0");
});

test("reports a signal kill as a failed result with a null exit code", async () => {
  const deps = makeDeps([
    { outcome: { exitCode: null, signal: "SIGTERM" }, stdout: "", stderr: "" },
  ], []);

  const result = await runWxpilot(deps, baseOptions());

  assert.equal(result.ok, false);
  assert.equal(result.exitCode, null);
  assert.match(result.text, /terminated by signal SIGTERM/);
});

test("keeps partial stdout from a signal-killed process", async () => {
  const deps = makeDeps([
    { outcome: { exitCode: null, signal: "SIGKILL" }, stdout: "partial output before kill", stderr: "" },
  ], []);

  const result = await runWxpilot(deps, baseOptions());

  assert.equal(result.ok, false);
  assert.equal(result.exitCode, null);
  assert.equal(result.data, null);
  assert.match(result.text, /partial output before kill/);
  assert.match(result.text, /terminated by signal SIGKILL/);
});

test("annotates a lossy collected stdout with the cap and spill path", async () => {
  const deps = makeDeps([
    {
      outcome: { exitCode: 0, signal: null },
      stdout: "tail of an oversized stream",
      stdoutLossy: true,
      stdoutSpillPath: "/var/tmp/wxpilot-stdout.spill",
    },
  ], []);

  const result = await runWxpilot(deps, baseOptions());

  assert.equal(result.data, null);
  assert.match(
    result.text,
    /\[stdout exceeded the in-memory collection cap; showing the retained tail only \(full output: \/var\/tmp\/wxpilot-stdout\.spill\)\]/,
  );
  assert.match(result.text, /tail of an oversized stream/);
});

test("truncates oversized rendered text with the CLI marker", async () => {
  const payload = JSON.stringify({ ok: true, blob: "y".repeat(500) });
  const deps = makeDeps([
    { outcome: { exitCode: 0, signal: null }, stdout: payload },
  ], []);

  const result = await runWxpilot(deps, baseOptions({ maxResultChars: 120 }));

  assert.ok(result.text.length < payload.length + 100);
  assert.match(result.text, /\[truncated, originalLength=\d+\]$/);
});

test("throws WXPILOT_LAUNCH_FAILED when the binary cannot be resolved", async () => {
  const deps: WxpilotProcessDeps = {
    async resolveExecutable() {
      throw new Error("not found");
    },
    spawn() {
      throw new Error("must not spawn");
    },
  };

  await assert.rejects(
    runWxpilot(deps, baseOptions()),
    (error: unknown) =>
      error instanceof WxpilotToolError &&
      error.code === "WXPILOT_LAUNCH_FAILED" &&
      /could not be resolved/.test(error.message),
  );
});

test("throws WXPILOT_ABORTED when the signal fires before the run", async () => {
  const controller = new AbortController();
  controller.abort();
  const deps = makeDeps([], []);

  await assert.rejects(
    runWxpilot(deps, baseOptions({ signal: controller.signal })),
    (error: unknown) =>
      error instanceof WxpilotToolError && error.code === "WXPILOT_ABORTED",
  );
});

test("createOperationQueue runs tasks in submission order", async () => {
  const enqueue = createOperationQueue();
  const order: string[] = [];
  const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

  const first = enqueue(async () => {
    await delay(20);
    order.push("first");
  });
  const second = enqueue(async () => {
    order.push("second");
  });

  await Promise.all([first, second]);
  assert.deepEqual(order, ["first", "second"]);
});

test("createOperationQueue keeps draining after a rejected task", async () => {
  const enqueue = createOperationQueue();
  const order: string[] = [];

  const rejected = enqueue(async () => {
    throw new Error("boom");
  });
  const followUp = enqueue(async () => {
    order.push("follow-up");
  });

  await assert.rejects(rejected, /boom/);
  await followUp;
  assert.deepEqual(order, ["follow-up"]);
});
