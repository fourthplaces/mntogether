import type { YogaInitialContext } from "graphql-yoga";
import { RestateClient } from "./restate-client";
import { createLoaders, type DataLoaders } from "./dataloaders";
import { parseCookie } from "./util";

export interface GraphQLContext {
  restate: RestateClient;
  loaders: DataLoaders;
}

export async function createContext(
  initialContext: YogaInitialContext
): Promise<GraphQLContext> {
  const cookieHeader =
    initialContext.request.headers.get("cookie") || "";
  const token = parseCookie(cookieHeader, "auth_token");

  const restate = new RestateClient({ token });

  return {
    restate,
    loaders: createLoaders(restate),
  };
}
