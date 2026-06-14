import { Suspense } from "react";
import { NoPermissionContent } from "./no-permission-content";

export default function NoPermissionPage() {
  return (
    <Suspense fallback={<main className="flex min-h-screen items-center justify-center text-text-muted">加载中...</main>}>
      <NoPermissionContent />
    </Suspense>
  );
}
