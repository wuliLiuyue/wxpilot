/**
 * Process-backed execution of the wxpilot Rust CLI through the `ctx.subprocess`
 * seam. The structural interfaces below mirror the seam's contract
 * (`@deepseek-ai/dsh-subprocess`): the real service satisfies them, and unit
 * tests supply fakes without the dsh runtime installed.
 *
 * Failure model: infrastructure failures (binary resolution, process launch,
 * missing output streams, caller abort) throw {@link WxpilotToolError}; every
 * domain outcome — including a non-zero CLI exit, a CLI-reported `ok:false`,
 * or a signal kill — is returned as a canonical result value.
 */

import { renderResultText, type WxpilotJsonObject } from "./render.js";

/** Collect cap for CLI stdout, aligned with the `@wxpilot/mcp` buffer size. */
export const STDOUT_MAX_BYTES = 2 * 1024 * 1024;
/** Collect cap for the retained CLI stderr diagnostic tail. */
export const STDERR_MAX_BYTES = 64 * 1024;
/** Terminate-escalation grace handed to the subprocess seam (ms). */
export const TERMINATE_GRACE_MS = 5_000;

/** Infrastructure failure codes surfaced by {@link WxpilotToolError}. */
export type WxpilotToolErrorCode =
  | "WXPILOT_ABORTED"
  | "WXPILOT_LAUNCH_FAILED"
  | "WXPILOT_NO_OUTPUT";

/** Infrastructure failure; thrown before or outside a CLI domain outcome. */
export class WxpilotToolError extends Error {
  constructor(
    message: string,
    readonly code: WxpilotToolErrorCode,
    options?: ErrorOptions,
  ) {
    super(message, options);
    this.name = "WxpilotToolError";
  }
}

/** One whole-stream collect read, mirroring the seam's reader shape. */
export interface CollectedRead {
  readonly text: string;
  readonly lossy: boolean;
  /** Path to the full-stream spill file, when the seam created one. */
  readonly spillPath?: string;
}

/** The spawn spec subset this tool needs; the seam applies no defaults. */
export interface WxpilotSpawnSpec {
  readonly argv: readonly string[];
  readonly cwd: string;
  readonly stdio: {
    readonly stdin: "ignore";
    readonly stdout: { readonly maxBytes: number };
    readonly stderr: { readonly maxBytes: number };
  };
  readonly graceMs: number;
  readonly signal: AbortSignal;
}

/** Process exit facts; `exitCode` is null when killed by a signal. */
export interface WxpilotOutcome {
  readonly exitCode: number | null;
  readonly signal: string | null;
}

/** Live handle subset: the awaited outcome plus collected output readers. */
export interface WxpilotHandle {
  readonly done: Promise<WxpilotOutcome>;
  readonly collected: {
    readonly stdout?: { readFrom(offset: number): CollectedRead };
    readonly stderr?: { readFrom(offset: number): CollectedRead };
  };
}

/** The seam surface this tool consumes. */
export interface WxpilotProcessDeps {
  resolveExecutable(
    command: string,
    env?: Readonly<Record<string, string>>,
    signal?: AbortSignal,
  ): Promise<string>;
  spawn(spec: WxpilotSpawnSpec): WxpilotHandle;
}

/** One execution request; `argv` is the full argument list after the binary,
 * including the leading `--json` flag produced by `buildCliArgv`. */
export interface WxpilotRunOptions {
  readonly binary: string;
  readonly argv: readonly string[];
  readonly cwd: string;
  readonly signal: AbortSignal;
  readonly maxResultChars: number;
}

/** Canonical result of one CLI run; `data` is the parsed `--json` payload. */
export interface WxpilotRunResult {
  readonly ok: boolean;
  readonly exitCode: number | null;
  readonly data: WxpilotJsonObject | null;
  readonly text: string;
}

/**
 * Run one wxpilot CLI invocation through the subprocess seam and reduce it to
 * the canonical result. `options.argv` must already carry the `--json` flag
 * (`buildCliArgv` produces exactly that shape) and is passed through verbatim
 * after the executable; the CLI's ordered JSON output (`ok` plus command
 * fields) becomes `data`.
 * @param deps - the seam surface (real service or test fake).
 * @param options - binary, argv, cwd, abort signal, and result budget.
 * @throws {WxpilotToolError} on abort, launch failure, or missing output streams.
 */
export async function runWxpilot(
  deps: WxpilotProcessDeps,
  options: WxpilotRunOptions,
): Promise<WxpilotRunResult> {
  if (options.signal.aborted) {
    throw abortedError();
  }
  let executable: string;
  try {
    executable = await deps.resolveExecutable(
      options.binary,
      undefined,
      options.signal,
    );
  } catch (error: unknown) {
    if (options.signal.aborted) {
      throw abortedError(error);
    }
    throw new WxpilotToolError(
      `wxpilot binary "${options.binary}" could not be resolved (is it installed and on PATH?)`,
      "WXPILOT_LAUNCH_FAILED",
      { cause: error },
    );
  }
  let handle: WxpilotHandle;
  try {
    handle = deps.spawn({
      argv: [executable, ...options.argv],
      cwd: options.cwd,
      stdio: {
        stdin: "ignore",
        stdout: { maxBytes: STDOUT_MAX_BYTES },
        stderr: { maxBytes: STDERR_MAX_BYTES },
      },
      graceMs: TERMINATE_GRACE_MS,
      signal: options.signal,
    });
  } catch (error: unknown) {
    if (options.signal.aborted) {
      throw abortedError(error);
    }
    throw new WxpilotToolError(
      "wxpilot process could not be started",
      "WXPILOT_LAUNCH_FAILED",
      { cause: error },
    );
  }
  let outcome: WxpilotOutcome;
  try {
    outcome = await handle.done;
  } catch (error: unknown) {
    if (options.signal.aborted) {
      throw abortedError(error);
    }
    throw new WxpilotToolError(
      "wxpilot process failed before exiting",
      "WXPILOT_LAUNCH_FAILED",
      { cause: error },
    );
  }
  const stdout = handle.collected.stdout?.readFrom(0);
  const stderr = handle.collected.stderr?.readFrom(0);
  if (stdout === undefined || stderr === undefined) {
    throw new WxpilotToolError(
      "wxpilot produced no collected output streams",
      "WXPILOT_NO_OUTPUT",
    );
  }
  if (options.signal.aborted) {
    throw abortedError();
  }
  if (outcome.exitCode === null || outcome.signal !== null) {
    return {
      ok: false,
      exitCode: outcome.exitCode,
      data: null,
      text: renderResultText(
        {
          parsed: null,
          stdout: lossyAnnotated("stdout", stdout),
          stderr: `wxpilot process was terminated by signal ${outcome.signal ?? "(unknown)"}\n${lossyAnnotated("stderr", stderr)}`,
          exitCode: outcome.exitCode,
        },
        options.maxResultChars,
      ),
    };
  }
  const parsed = parseJsonPayload(stdout.text);
  return {
    ok: parsed?.ok === true,
    exitCode: outcome.exitCode,
    data: parsed,
    text: renderResultText(
      {
        parsed,
        stdout: lossyAnnotated("stdout", stdout),
        stderr: lossyAnnotated("stderr", stderr),
        exitCode: outcome.exitCode,
      },
      options.maxResultChars,
    ),
  };
}

/**
 * Surface a lossy collected read: the seam's in-memory cap keeps the TAIL of
 * an overflowing stream, so the retained text is annotated (and the spill
 * path, when one exists, is surfaced for manual recovery) instead of being
 * mistaken for the complete output.
 */
function lossyAnnotated(label: string, read: CollectedRead): string {
  if (!read.lossy) {
    return read.text;
  }
  const spill = read.spillPath === undefined ? "" : ` (full output: ${read.spillPath})`;
  return `[${label} exceeded the in-memory collection cap; showing the retained tail only${spill}]\n${read.text}`;
}

/**
 * Serialize operations through one promise chain. The CLI drives a stateful
 * DevTools session (`view` assigns `%N` refs that later operations consume),
 * so concurrent invocations are serialized in submission order.
 */
export function createOperationQueue(): <T>(task: () => Promise<T>) => Promise<T> {
  let tail: Promise<void> = Promise.resolve();
  return (task) => {
    const result = tail.then(task, task);
    tail = result.then(
      () => undefined,
      () => undefined,
    );
    return result;
  };
}

/** Decode the CLI's `--json` payload; non-object or unparsable text is null. */
function parseJsonPayload(text: string): WxpilotJsonObject | null {
  const trimmed = text.trim();
  if (!trimmed.startsWith("{")) {
    return null;
  }
  try {
    const value: unknown = JSON.parse(trimmed);
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
      return null;
    }
    return value as WxpilotJsonObject;
  } catch {
    return null;
  }
}

function abortedError(cause?: unknown): WxpilotToolError {
  return new WxpilotToolError(
    "wxpilot call was aborted (tool timeout or caller cancellation)",
    "WXPILOT_ABORTED",
    { cause },
  );
}
