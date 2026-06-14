"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { fetchApi } from "@/lib/api-client";

interface TopBarProps {
  portalName?: string;
  displayName?: string;
  tenantName?: string;
  canEnterAdmin?: boolean;
  isAdmin?: boolean;
}

export function TopBar({
  portalName = "企业门户",
  displayName = "",
  tenantName = "",
  canEnterAdmin = false,
  isAdmin = false,
}: TopBarProps) {
  const router = useRouter();

  async function logout() {
    try {
      await fetchApi<unknown>("/api/auth/logout", { method: "POST" });
    } catch {
      // ignore logout errors and redirect anyway
    }
    router.push("/login");
    router.refresh();
  }

  return (
    <header className="flex h-[60px] items-center justify-between border-b border-border-subtle bg-bg-secondary px-6">
      <div className="flex items-center gap-4">
        <span className="text-base font-semibold text-text-primary">{portalName}</span>
        {tenantName && (
          <span className="rounded-radius-sm bg-bg-tertiary px-2.5 py-1 text-xs text-text-secondary">
            {tenantName}
          </span>
        )}
      </div>
      <div className="flex items-center gap-3">
        {canEnterAdmin && !isAdmin && (
          <Button variant="ghost" asChild>
            <Link href="/admin">管理后台</Link>
          </Button>
        )}
        {isAdmin && (
          <Button variant="ghost" asChild>
            <Link href="/">门户首页</Link>
          </Button>
        )}
        <span className="text-sm text-text-secondary">{displayName}</span>
        <Button variant="ghost" onClick={logout}>
          退出
        </Button>
      </div>
    </header>
  );
}
