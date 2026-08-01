import { execFile } from "node:child_process";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const DEFAULT_TIMEOUT_MS = 120_000;
const MAX_BUFFER_BYTES = 2 * 1024 * 1024;

export interface InvokeOptions {
  env?: NodeJS.ProcessEnv;
  binary?: string;
  cwd?: string;
  timeout?: number;
}

export interface InvokeResult {
  ok: boolean;
  text: string;
}

export async function invokeWxpilot(
  argv: readonly string[],
  options: InvokeOptions = {},
): Promise<InvokeResult> {
  const env = options.env ?? process.env;
  const binary = options.binary ?? env.WXPILOT_BIN ?? "wxpilot";
  const cwd = options.cwd ?? env.WXPILOT_CWD ?? undefined;
  const timeout =
    options.timeout ?? readTimeout(env.WXPILOT_MCP_PROCESS_TIMEOUT_MS);

  try {
    const result = await execFileAsync(binary, [...argv], {
      cwd,
      env: { ...env },
      killSignal: "SIGTERM",
      maxBuffer: MAX_BUFFER_BYTES,
      timeout,
    });

    return {
      ok: true,
      text: mergeOutput(result.stdout, result.stderr) || '{"ok":true}',
    };
  } catch (error: unknown) {
    const details = getErrorDetails(error);
    return {
      ok: false,
      text:
        mergeOutput(details.stdout, details.stderr) ||
        details.message ||
        "wxpilot command failed",
    };
  }
}

function readTimeout(raw: string | undefined): number {
  if (raw === undefined) {
    return DEFAULT_TIMEOUT_MS;
  }
  const value = Number(raw);
  if (!Number.isInteger(value) || value < 0) {
    return DEFAULT_TIMEOUT_MS;
  }
  return value;
}

function mergeOutput(stdout: unknown, stderr: unknown): string {
  return [stdout, stderr]
    .filter((value): value is string => typeof value === "string" && value.trim().length > 0)
    .map((value) => value.trim())
    .join("\n");
}

interface ErrorDetails {
  stdout?: unknown;
  stderr?: unknown;
  message?: string;
}

function getErrorDetails(error: unknown): ErrorDetails {
  if (error instanceof Error) {
    const details = error as Error & {
      stdout?: unknown;
      stderr?: unknown;
    };
    return {
      stdout: details.stdout,
      stderr: details.stderr,
      message: details.message,
    };
  }
  return { message: String(error) };
}
