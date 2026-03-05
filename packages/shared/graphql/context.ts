import type { YogaInitialContext } from "graphql-yoga";
import { ServerClient } from "./server-client";
import { createLoaders, type DataLoaders } from "./dataloaders";
import { parseCookie } from "./util";

export interface GraphQLContext {
  server: ServerClient;
  loaders: DataLoaders;
}

export async function createContext(
  initialContext: YogaInitialContext
): Promise<GraphQLContext> {
  const cookieHeader =
    initialContext.request.headers.get("cookie") || "";
  const token = parseCookie(cookieHeader, "auth_token");

  const server = new ServerClient({ token });

  return {
    server,
    loaders: createLoaders(server),
  };
}
