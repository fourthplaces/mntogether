"use client";

import { Component, type ReactNode } from "react";
import { Button } from "@/components/ui/button";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

export class GraphQLErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  render() {
    if (this.state.error) {
      return (
        <div className="flex flex-col items-center justify-center py-24 px-6">
          <div className="max-w-md text-center">
            <div className="text-4xl mb-4">!</div>
            <h2 className="text-lg font-semibold text-foreground mb-2">
              Something went wrong
            </h2>
            <p className="text-sm text-muted-foreground mb-6">
              {this.state.error.message || "An unexpected error occurred."}
            </p>
            <Button
              variant="admin"
              onClick={() => this.setState({ error: null })}
            >
              Try again
            </Button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
