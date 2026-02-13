import { mergeResolvers } from "@graphql-tools/merge";
import { postResolvers } from "./post";
import { tagResolvers } from "./tag";

export const resolvers = mergeResolvers([postResolvers, tagResolvers]);
