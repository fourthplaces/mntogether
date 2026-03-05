import type { ServerClient } from "../server-client";
import { createPostLoaders, type PostLoaders } from "./post";

export interface DataLoaders extends PostLoaders {}

export function createLoaders(server: ServerClient): DataLoaders {
  return {
    ...createPostLoaders(server),
  };
}
