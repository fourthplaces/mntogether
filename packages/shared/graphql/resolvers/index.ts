import { mergeResolvers } from "@graphql-tools/merge";
import { postResolvers } from "./post";
import { tagResolvers } from "./tag";
import { organizationResolvers } from "./organization";

export const resolvers = mergeResolvers([
  postResolvers,
  tagResolvers,
  organizationResolvers,
]);
