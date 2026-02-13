import { mergeResolvers } from "@graphql-tools/merge";
import { postResolvers } from "./post";
import { tagResolvers } from "./tag";
import { organizationResolvers } from "./organization";
import { sourceResolvers } from "./source";

export const resolvers = mergeResolvers([
  postResolvers,
  tagResolvers,
  organizationResolvers,
  sourceResolvers,
]);
