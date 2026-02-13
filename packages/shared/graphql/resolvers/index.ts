import { mergeResolvers } from "@graphql-tools/merge";
import { postResolvers } from "./post";
import { tagResolvers } from "./tag";
import { organizationResolvers } from "./organization";
import { sourceResolvers } from "./source";
import { websiteResolvers } from "./website";
import { syncResolvers } from "./sync";

export const resolvers = mergeResolvers([
  postResolvers,
  tagResolvers,
  organizationResolvers,
  sourceResolvers,
  websiteResolvers,
  syncResolvers,
]);
