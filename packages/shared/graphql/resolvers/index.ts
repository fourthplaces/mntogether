import { mergeResolvers } from "@graphql-tools/merge";
import { postResolvers } from "./post";
import { tagResolvers } from "./tag";
import { organizationResolvers } from "./organization";
import { jobResolvers } from "./job";
import { noteResolvers } from "./note";

export const resolvers = mergeResolvers([
  postResolvers,
  tagResolvers,
  organizationResolvers,
  jobResolvers,
  noteResolvers,
]);
