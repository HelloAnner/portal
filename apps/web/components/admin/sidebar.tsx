"use client";

import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import {
  AppWindow,
  Bell,
  Building2,
  Database,
  FileText,
  Hexagon,
  Home,
  LayoutDashboard,
  LayoutGrid,
  LogOut,
  Plug,
  ScrollText,
  Search,
  Shield,
  ShieldCheck,
  User,
  UserCircle,
  UserCog,
  Users,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { fetchApi } from "@/lib/api-client";

const menuItems = [
  { href: "/", label: "首页", icon: Home },
  { href: "/#systems", label: "全部系统", icon: LayoutGrid },
  { href: "/#northline", label: "Northline", icon: Database },
  { href: "/#documind", label: "DocuMind", icon: FileText },
  { href: "/profile", label: "我的资料", icon: User },
  { href: "/permissions", label: "我的权限", icon: Shield },
];

const adminItems = [
  { href: "/admin", label: "概览", icon: LayoutDashboard },
  { href: "/admin/users", label: "用户管理", icon: Users },
  { href: "/admin/tenants", label: "租户管理", icon: Building2 },
  { href: "/admin/systems", label: "系统目录", icon: AppWindow },
  { href: "/admin/permissions", label: "权限配置", icon: ShieldCheck },
  { href: "/admin/roles", label: "角色管理", icon: UserCircle },
  { href: "/admin/sub-admins", label: "子系统管理员", icon: UserCog },
  { href: "/admin/integrations", label: "子系统接入", icon: Plug },
];

const securityItems = [
  { href: "/admin/audits", label: "安全与审计", icon: ScrollText },
];

interface AdminSidebarProps {
  showAdmin?: boolean;
  displayName?: string;
  roleLabel?: string;
}

function isActive(pathname: string, href: string) {
  if (href === "/") return pathname === "/";
  if (href.includes("#")) return pathname === "/";
  return pathname === href || pathname.startsWith(`${href}/`);
}

export function AdminSidebar({
  showAdmin = true,
  displayName = "Portal User",
  roleLabel = "User",
}: AdminSidebarProps) {
  const pathname = usePathname();
  const router = useRouter();

  async function logout() {
    try {
      await fetchApi<unknown>("/api/auth/logout", { method: "POST" });
    } catch {
      // Always leave the local UI after logout is requested.
    }
    router.push("/login");
    router.refresh();
  }

  function renderGroup(title: string, items: typeof menuItems) {
    return (
      <div className="space-y-1">
        <div className="px-3 py-2 text-[11px] font-semibold uppercase tracking-wide text-text-muted">
          {title}
        </div>
        {items.map((item) => {
          const active = isActive(pathname, item.href);
          const Icon = item.icon;
          return (
            <Link
              key={`${title}-${item.href}`}
              href={item.href}
              className={cn(
                "flex h-10 items-center gap-3 rounded-radius-sm px-3 text-sm transition-colors",
                active
                  ? "bg-selected-bg text-primary"
                  : "text-text-secondary hover:bg-hover-bg hover:text-text-primary"
              )}
            >
              <Icon className="h-5 w-5 shrink-0" />
              <span className="truncate">{item.label}</span>
            </Link>
          );
        })}
      </div>
    );
  }

  return (
    <aside className="sticky top-0 flex h-screen w-[260px] shrink-0 flex-col bg-bg-secondary p-4">
      <Link href="/" className="flex h-11 items-center gap-2.5 rounded-radius-sm px-3 text-text-primary">
        <Hexagon className="h-6 w-6 text-primary" />
        <span className="text-lg font-semibold">Portal</span>
      </Link>

      <nav className="mt-3 flex-1 space-y-4 overflow-y-auto pb-4">
        {renderGroup("Menu", menuItems)}
        {showAdmin && renderGroup("Admin", adminItems)}
        {showAdmin && renderGroup("Security", securityItems)}
      </nav>

      <div className="space-y-2 border-t border-border-faint pt-3">
        <div className="flex items-center gap-3 rounded-radius-sm px-3 py-2">
          <div className="flex h-8 w-8 items-center justify-center rounded-full bg-bg-primary text-sm font-semibold text-primary">
            {displayName.slice(0, 1) || "P"}
          </div>
          <div className="min-w-0 flex-1">
            <div className="truncate text-sm text-text-primary">{displayName}</div>
            <div className="text-xs text-text-muted">{roleLabel}</div>
          </div>
        </div>
        <div className="grid grid-cols-3 gap-1">
          <button
            type="button"
            className="flex h-9 items-center justify-center rounded-radius-sm text-text-muted hover:bg-hover-bg hover:text-text-primary"
            aria-label="搜索"
          >
            <Search className="h-4 w-4" />
          </button>
          <button
            type="button"
            className="flex h-9 items-center justify-center rounded-radius-sm text-text-muted hover:bg-hover-bg hover:text-text-primary"
            aria-label="通知"
          >
            <Bell className="h-4 w-4" />
          </button>
          <button
            type="button"
            onClick={logout}
            className="flex h-9 items-center justify-center rounded-radius-sm text-text-muted hover:bg-hover-bg hover:text-text-primary"
            aria-label="退出"
          >
            <LogOut className="h-4 w-4" />
          </button>
        </div>
      </div>
    </aside>
  );
}
