import { Suspense } from "react";
import { LoginForm } from "./LoginForm";

function LoginFormSkeleton() {
  return (
    <div className="bg-white rounded-lg shadow-md p-8 max-w-md w-full animate-pulse">
      <div className="mb-6 text-center">
        <div className="h-8 bg-gray-200 rounded w-32 mx-auto mb-2"></div>
        <div className="h-4 bg-gray-200 rounded w-24 mx-auto"></div>
      </div>
      <div className="mb-4">
        <div className="h-4 bg-gray-200 rounded w-40 mb-2"></div>
        <div className="h-10 bg-gray-200 rounded"></div>
      </div>
      <div className="h-10 bg-gray-200 rounded"></div>
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
