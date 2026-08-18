export const OPERATIONS = Object.freeze([
  "start",
  "connect",
  "disconnect",
  "status",
  "goto",
  "back",
  "reload",
  "view",
  "find",
  "wait",
  "tap",
  "type",
  "scroll",
  "read",
  "assert",
  "state",
  "shot",
  "run",
  "wx",
  "net.start",
  "net.stop",
  "net.log",
  "net.mock",
  "net.unmock",
  "net.clear",
  "net.install-ca",
] as const);

export type Operation = (typeof OPERATIONS)[number];

const OPERATION_SET = new Set<string>(OPERATIONS);

export class CommandValidationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "CommandValidationError";
  }
}

/**
 * Build the wxpilot CLI argument list for one operation.
 *
 * Contract: the returned array is the COMPLETE argument list after the binary
 * — it starts with the global `--json` flag and both adapters must forward it
 * verbatim (the CLI rejects a repeated `--json`). Consumers that prepend
 * anything must not inject `--json` again.
 */
export function buildCliArgv(operation: string, input?: unknown): string[] {
  if (!OPERATION_SET.has(operation)) {
    throw new CommandValidationError(`unsupported operation: ${operation}`);
  }

  const params = normalizeParams(input);
  const argv = ["--json"];

  switch (operation as Operation) {
    case "start": {
      const projectPath = requiredString(params, "projectPath");
      argv.push("start");
      appendOption(argv, "--cli-path", optionalString(params, "cliPath"));
      appendOption(argv, "--appid", optionalString(params, "appid"));
      appendOption(
        argv,
        "--auto-port",
        optionalInteger(params, "autoPort", undefined, 0, 65535),
      );
      appendFlag(argv, "--proxy", optionalBoolean(params, "proxy", false));
      appendFlag(argv, "--https", optionalBoolean(params, "https", false));
      appendPositionals(argv, projectPath);
      return argv;
    }

    case "connect": {
      argv.push("connect");
      appendOption(argv, "--ws", optionalString(params, "wsEndpoint"));
      return argv;
    }

    case "disconnect":
    case "status":
    case "back":
    case "reload":
      argv.push(operation);
      return argv;

    case "goto": {
      appendGlobalTimeout(argv, params);
      argv.push("goto");
      appendPositionals(argv, requiredString(params, "route"));
      return argv;
    }

    case "view": {
      argv.push("view");
      appendFlag(argv, "--full", optionalBoolean(params, "full", false));
      const compact = optionalBoolean(params, "compact", undefined);
      if (compact !== undefined) {
        argv.push(compact ? "--compact" : "--compact=false");
      }
      appendOption(
        argv,
        "--depth",
        optionalInteger(params, "depth", undefined, 0, 1000),
      );
      appendOption(
        argv,
        "--limit",
        optionalInteger(params, "limit", undefined, 0, 1_000_000),
      );
      return argv;
    }

    case "find": {
      argv.push("find");
      appendFlag(argv, "--all", optionalBoolean(params, "all", false));
      appendOption(
        argv,
        "--limit",
        optionalInteger(params, "limit", undefined, 0, 1_000_000),
      );
      appendPositionals(argv, requiredString(params, "query"));
      return argv;
    }

    case "wait": {
      argv.push("wait");
      const timeout = optionalInteger(
        params,
        "timeoutMs",
        undefined,
        1,
        600_000,
      );
      appendOption(argv, "--timeout", timeout);
      appendPositionals(argv, requiredString(params, "reference"));
      return argv;
    }

    case "tap":
    case "read": {
      argv.push(operation);
      appendPositionals(argv, requiredString(params, "reference"));
      return argv;
    }

    case "type": {
      argv.push("type");
      appendPositionals(
        argv,
        requiredString(params, "reference"),
        requiredString(params, "text"),
      );
      return argv;
    }

    case "scroll": {
      const direction = requiredString(params, "direction");
      if (direction !== "up" && direction !== "down") {
        throw new CommandValidationError("direction must be up or down");
      }
      argv.push("scroll");
      appendPositionals(argv, requiredString(params, "reference"), direction);
      return argv;
    }

    case "assert": {
      argv.push("assert");
      appendPositionals(
        argv,
        requiredString(params, "reference"),
        requiredString(params, "expected"),
      );
      return argv;
    }

    case "state": {
      argv.push("state");
      appendFlag(argv, "--full", optionalBoolean(params, "full", false));
      const path = optionalString(params, "path");
      if (path !== undefined) {
        appendPositionals(argv, path);
      }
      return argv;
    }

    case "shot": {
      argv.push("shot");
      appendFlag(argv, "--base64", optionalBoolean(params, "base64", false));
      const path = optionalString(params, "path");
      if (path !== undefined) {
        appendPositionals(argv, path);
      }
      return argv;
    }

    case "run":
      argv.push("run");
      appendPositionals(argv, requiredString(params, "js"));
      return argv;

    case "wx": {
      const method = requiredString(params, "method");
      const args = params.args === undefined ? [] : params.args;
      if (!Array.isArray(args)) {
        throw new CommandValidationError("params.args must be an array");
      }
      argv.push("wx", "--", method, ...args.map(serializeJsonArgument));
      return argv;
    }

    case "net.start": {
      argv.push("net", "start");
      appendOption(
        argv,
        "--port",
        optionalInteger(params, "port", undefined, 0, 65535),
      );
      appendFlag(argv, "--https", optionalBoolean(params, "https", false));
      return argv;
    }

    case "net.stop":
      argv.push("net", "stop");
      return argv;

    case "net.log": {
      argv.push("net", "log");
      appendOption(argv, "--filter", optionalString(params, "filter"));
      appendOption(
        argv,
        "--limit",
        optionalInteger(params, "limit", undefined, 0, 1_000_000),
      );
      appendFlag(
        argv,
        "--with-body",
        optionalBoolean(params, "withBody", false),
      );
      return argv;
    }

    case "net.mock": {
      argv.push("net", "mock");
      appendOption(argv, "--method", optionalString(params, "method"));
      appendPositionals(
        argv,
        requiredString(params, "url"),
        requiredString(params, "jsonFile"),
      );
      return argv;
    }

    case "net.unmock":
      argv.push("net", "unmock");
      appendPositionals(argv, requiredString(params, "url"));
      return argv;

    case "net.clear":
      argv.push("net", "clear");
      return argv;

    case "net.install-ca":
      argv.push("net", "install-ca");
      return argv;

    default:
      throw new CommandValidationError(`unsupported operation: ${operation}`);
  }
}

function normalizeParams(input: unknown): Record<string, unknown> {
  if (input === undefined) {
    return {};
  }
  if (input === null || typeof input !== "object" || Array.isArray(input)) {
    throw new CommandValidationError("params must be an object");
  }
  return input as Record<string, unknown>;
}

function requiredString(params: Record<string, unknown>, key: string): string {
  const value = params[key];
  if (typeof value !== "string" || value.length === 0) {
    throw new CommandValidationError(`params.${key} must be a non-empty string`);
  }
  return value;
}

function optionalString(
  params: Record<string, unknown>,
  key: string,
): string | undefined {
  const value = params[key];
  if (value === undefined) {
    return undefined;
  }
  if (typeof value !== "string" || value.length === 0) {
    throw new CommandValidationError(`params.${key} must be a non-empty string`);
  }
  return value;
}

function optionalBoolean(
  params: Record<string, unknown>,
  key: string,
  fallback: boolean | undefined,
): boolean | undefined {
  const value = params[key];
  if (value === undefined) {
    return fallback;
  }
  if (typeof value !== "boolean") {
    throw new CommandValidationError(`params.${key} must be a boolean`);
  }
  return value;
}

function optionalInteger(
  params: Record<string, unknown>,
  key: string,
  fallback: number | undefined,
  min: number,
  max: number,
): number | undefined {
  const value = params[key];
  if (value === undefined) {
    return fallback;
  }
  if (typeof value !== "number" || !Number.isInteger(value) || value < min || value > max) {
    throw new CommandValidationError(
      `params.${key} must be an integer between ${min} and ${max}`,
    );
  }
  return value;
}

function appendFlag(argv: string[], flag: string, enabled: boolean | undefined): void {
  if (enabled) {
    argv.push(flag);
  }
}

function appendOption(
  argv: string[],
  flag: string,
  value: string | number | undefined,
): void {
  if (value !== undefined) {
    argv.push(flag, String(value));
  }
}

function appendPositionals(argv: string[], ...values: string[]): void {
  argv.push("--", ...values);
}

function appendGlobalTimeout(
  argv: string[],
  params: Record<string, unknown>,
): void {
  appendOption(
    argv,
    "--timeout",
    optionalInteger(params, "timeoutMs", undefined, 1, 600_000),
  );
}

function serializeJsonArgument(value: unknown): string {
  const serialized = JSON.stringify(value);
  if (serialized === undefined) {
    throw new CommandValidationError("params.args contains an unsupported value");
  }
  return serialized;
}
