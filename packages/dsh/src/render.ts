/**
 * Model-facing text rendering for wxpilot CLI output. The marker format
 * matches the CLI's own truncation convention (`[truncated, originalLength=N]`)
 * so downstream readers treat both layers the same way.
 */

/** Lossless JSON value, structurally compatible with dsh's `JsonValue`. */
export type WxpilotJson = null | boolean | number | string | WxpilotJson[] | { [key: string]: WxpilotJson };

/** A decoded JSON object. */
export type WxpilotJsonObject = { [key: string]: WxpilotJson };

/** Inputs of one rendered result; `parsed` is the decoded `--json` payload. */
export interface RenderResultInput {
  readonly parsed: WxpilotJsonObject | null;
  readonly stdout: string;
  readonly stderr: string;
  readonly exitCode: number | null;
}

/**
 * Truncate text to `maxChars`, appending the CLI truncation marker when cut.
 * @param text - complete text to bound.
 * @param maxChars - maximum retained character count, marker excluded.
 */
export function truncateWithMarker(text: string, maxChars: number): string {
  if (text.length <= maxChars) {
    return text;
  }
  return `${text.slice(0, maxChars)}\n[truncated, originalLength=${text.length}]`;
}

/**
 * Render one CLI run as the model-facing `text` field: the pretty-printed JSON
 * payload when present, otherwise the raw stdout/stderr fallback; on failure,
 * a non-duplicated stderr tail is appended.
 * @param input - the run's parsed payload and raw streams.
 * @param maxChars - maximum retained character count for the final text.
 */
export function renderResultText(
  input: RenderResultInput,
  maxChars: number,
): string {
  const parts: string[] = [];
  if (input.parsed !== null) {
    parts.push(JSON.stringify(input.parsed, null, 2));
  } else {
    parts.push(
      input.stdout.trim() ||
        input.stderr.trim() ||
        `wxpilot exited with code ${input.exitCode ?? "unknown"}`,
    );
  }
  const failed = input.parsed === null || input.parsed.ok !== true;
  const stderrText = input.stderr.trim();
  // `parts[0]` already IS the stderr fallback when stdout was empty; a strict
  // equality avoids dropping a stderr that merely appears as a substring of
  // the stdout fallback (e.g. a short stderr echoed inside a larger payload).
  if (failed && stderrText.length > 0 && parts[0] !== stderrText) {
    parts.push(`stderr:\n${stderrText}`);
  }
  return truncateWithMarker(parts.join("\n"), maxChars);
}
