import { Suspense } from "react";
import { LoginForm } from "./LoginForm";
import { Skeleton } from "@/components/ui/skeleton";

function LoginFormSkeleton() {
  return (
    <div className="bg-card rounded-lg shadow-md p-8 max-w-md w-full">
      <div className="mb-6 text-center space-y-2">
        <Skeleton className="h-8 w-32 mx-auto" />
        <Skeleton className="h-4 w-24 mx-auto" />
      </div>
      <div className="mb-4 space-y-2">
        <Skeleton className="h-4 w-40" />
        <Skeleton className="h-10 w-full" />
      </div>
      <Skeleton className="h-10 w-full" />
    </div>
  );
}

export default function LoginPage() {
  return (
    <div className="min-h-screen bg-amber-50 flex items-center justify-center p-4">
      <Suspense fallback={<LoginFormSkeleton />}>
        <LoginForm />
      </Suspense>
    </div>
  );
}
