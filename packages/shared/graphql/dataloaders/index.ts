import type { RestateClient } from "../restate-client";
import { createPostLoaders, type PostLoaders } from "./post";

export interface DataLoaders extends PostLoaders {}

export function createLoaders(restate: RestateClient): DataLoaders {
  return {
    ...createPostLoaders(restate),
  };
}
