"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { cn } from "@/lib/utils";

const navItems = [
  { href: "/admin", label: "概览" },
  { href: "/admin/users", label: "用户管理" },
  { href: "/admin/tenants", label: "租户管理" },
  { href: "/admin/roles", label: "角色管理" },
  { href: "/admin/systems", label: "系统目录" },
  { href: "/admin/permissions", label: "权限配置" },
  { href: "/admin/sub-admins", label: "子系统管理员" },
  { href: "/admin/integrations", label: "子系统接入" },
  { href: "/admin/audits", label: "安全审计" },
];

export function AdminSidebar() {
  const pathname = usePathname();

  return (
    <aside className="flex h-screen w-60 flex-col border-r border-border-subtle bg-bg-tertiary">
      <div className="flex h-[60px] items-center px-5">
        <span className="text-sm font-semibold uppercase tracking-wider text-text-muted">管理后台</span>
      </div>
      <nav className="flex-1 space-y-1 px-3 py-2">
        {navItems.map((item) => {
          const active = pathname === item.href || pathname.startsWith(`${item.href}/`);
          return (
            <Link
              key={item.href}
              href={item.href}
              className={cn(
                "block rounded-radius-sm px-3 py-2 text-sm transition-colors",
                active
                  ? "bg-selected-bg text-text-primary"
                  : "text-text-secondary hover:bg-hover-bg hover:text-text-primary"
              )}
            >
              {item.label}
            </Link>
          );
        })}
      </nav>
    </aside>
  );
}
