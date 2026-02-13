import { GraphQLError } from "graphql";
import { snakeToCamel } from "./util";

const RESTATE_INGRESS_URL =
  process.env.RESTATE_INGRESS_URL || "http://localhost:8180";
const RESTATE_AUTH_TOKEN = process.env.RESTATE_AUTH_TOKEN || "";

export class RestateClient {
  private token: string | null;

  constructor({ token }: { token: string | null }) {
    this.token = token;
  }

  async callService<T>(
    service: string,
    handler: string,
    body?: unknown
  ): Promise<T> {
    return this.call<T>(`${service}/${handler}`, body);
  }

  async callObject<T>(
    object: string,
    key: string,
    handler: string,
    body?: unknown
  ): Promise<T> {
    return this.call<T>(`${object}/${key}/${handler}`, body);
  }

  private async call<T>(path: string, body?: unknown): Promise<T> {
    const headers: HeadersInit = { "Content-Type": "application/json" };
    if (RESTATE_AUTH_TOKEN)
      headers["Authorization"] = `Bearer ${RESTATE_AUTH_TOKEN}`;
    if (this.token) headers["X-User-Token"] = this.token;

    const response = await fetch(`${RESTATE_INGRESS_URL}/${path}`, {
      method: "POST",
      headers,
      body: JSON.stringify(body ?? {}),
    });

    if (!response.ok) {
      const text = await response.text();
      let message: string;
      try {
        const json = JSON.parse(text);
        message = json.message || json.error || text;
      } catch {
        message = text;
      }
      throw new GraphQLError(message, {
        extensions: {
          code:
            response.status === 401
              ? "UNAUTHENTICATED"
              : response.status === 403
                ? "FORBIDDEN"
                : response.status === 404
                  ? "NOT_FOUND"
                  : "INTERNAL_SERVER_ERROR",
        },
      });
    }

    const raw = await response.json();
    return snakeToCamel(raw) as T;
  }
}
