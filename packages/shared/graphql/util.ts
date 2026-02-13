/**
 * Deep transform all keys in an object from snake_case to camelCase.
 * Handles nested objects, arrays, and null values.
 */
export function snakeToCamel(obj: unknown): unknown {
  if (obj === null || obj === undefined) return obj;
  if (Array.isArray(obj)) return obj.map(snakeToCamel);
  if (typeof obj !== "object") return obj;

  const result: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(obj as Record<string, unknown>)) {
    const camelKey = key.replace(/_([a-z])/g, (_, letter) =>
      letter.toUpperCase()
    );
    result[camelKey] = snakeToCamel(value);
  }
  return result;
}

/**
 * Parse a specific cookie from a cookie header string.
 */
export function parseCookie(
  cookieHeader: string,
  name: string
): string | null {
  const match = cookieHeader.match(
    new RegExp(`(?:^|;\\s*)${name}=([^;]*)`)
  );
  return match ? decodeURIComponent(match[1]) : null;
}
