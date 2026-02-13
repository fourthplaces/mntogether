import { NextRequest } from "next/server";
import { createYoga } from "graphql-yoga";
import { schema, createContext } from "@mntogether/shared";

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
