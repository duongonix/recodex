type ClientErrorInput = {
  error: unknown;
  event: { url: { pathname: string } };
  kind: "app" | "framework" | "unknown";
  status: number;
};

export function handleError({ error, event, kind, status }: ClientErrorInput): { message: string } | void {
  if (kind !== "unknown" || status < 500 || event.url.pathname !== "/") return;

  try {
    const detail = error instanceof Error
      ? `${error.name}: ${error.message}`
      : typeof error === "string" ? error : "A non-Error value was thrown.";
    const safeDetail = detail
      .replace(/(?:[a-z]:\\|\\\\)[^\r\n"'<>]*/gi, "[local path]")
      .replace(/file:\/\/\/[^\s"'<>]*/gi, "[local file]")
      .slice(0, 240);

    // SvelteKit hides unknown client errors behind "Internal Error" in production.
    // Expose a short message with local paths redacted and no stack trace.
    return { message: `RecoDex could not load: ${safeDetail}` };
  } catch {
    // SvelteKit requires this hook to be non-throwing.
    return { message: "RecoDex could not load. Restart the app and try again." };
  }
}
