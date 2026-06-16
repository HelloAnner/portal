"use client";

import Link from "next/link";
import { Bell, Search } from "lucide-react";
import { Button } from "@/components/ui/button";

interface TopBarProps {
  title?: string;
  displayName?: string;
  tenantName?: string;
  canEnterAdmin?: boolean;
  isAdmin?: boolean;
}

export function TopBar({
  title = "Portal",
  displayName = "",
  tenantName = "",
  canEnterAdmin = false,
  isAdmin = false,
}: TopBarProps) {
  return (
    <header className="flex h-16 items-center justify-between bg-bg-primary px-8">
      <div className="flex items-center gap-4">
        <span className="text-sm font-semibold text-text-primary">{title}</span>
        {tenantName && (
          <span className="rounded-full bg-bg-secondary px-3 py-1.5 text-xs text-text-secondary">
            {tenantName}
          </span>
        )}
      </div>
      <div className="flex items-center gap-3">
        <div className="hidden h-9 w-64 items-center gap-2 rounded-full border border-border-faint bg-bg-secondary px-3 text-xs text-text-muted md:flex">
          <Search className="h-3.5 w-3.5" />
          <span>Search Portal</span>
        </div>
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
        <button
          type="button"
          className="flex h-9 w-9 items-center justify-center rounded-full bg-bg-secondary text-text-muted hover:text-text-primary"
          aria-label="通知"
        >
          <Bell className="h-4 w-4" />
        </button>
        <span className="hidden text-sm text-text-secondary sm:inline">{displayName}</span>
      </div>
    </header>
  );
}
