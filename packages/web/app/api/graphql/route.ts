import { NextRequest } from "next/server";
import { createYoga, createSchema } from "graphql-yoga";
import { typeDefs, resolvers, createContext } from "@mntogether/shared";

const schema = createSchema({ typeDefs, resolvers });

const yoga = createYoga({
  schema,
  context: createContext,
  graphqlEndpoint: "/api/graphql",
  fetchAPI: { Response },
});

export async function GET(request: NextRequest) {
  return yoga.handleRequest(request, {});
}

export async function POST(request: NextRequest) {
  return yoga.handleRequest(request, {});
}
