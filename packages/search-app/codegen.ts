import type { CodegenConfig } from "@graphql-codegen/cli";

const config: CodegenConfig = {
  schema: "../shared/graphql/typeDefs/**/*.graphql",
  documents: [
    "./app/**/*.tsx",
    "./app/**/*.ts",
    "./components/**/*.tsx",
    "./components/**/*.ts",
    "./lib/**/*.ts",
    "!./gql/**/*",
  ],
  ignoreNoDocuments: true,
  generates: {
    "./gql/": {
      preset: "client",
      presetConfig: { fragmentMasking: false },
      config: {
        scalars: { DateTime: "string", UUID: "string" },
      },
    },
  },
};

export default config;
