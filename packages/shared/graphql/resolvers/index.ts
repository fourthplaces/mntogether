import { mergeResolvers } from "@graphql-tools/merge";
import { postResolvers } from "./post";

export const resolvers = mergeResolvers([postResolvers]);
