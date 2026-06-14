"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { fetchApi, handleApiError } from "@/lib/api-client";

interface OverviewData {
  stats: {
    userTotal: number;
    activeSystemTotal: number;
    portalManagedSystemTotal: number;
    subsystemEntry24h: number;
    highRiskPermissionChanges24h: number;
  };
  todos: { type: string; title: string; targetType: string; targetId: string }[];
  recentAudits: any[];
}

const statCards = [
  { key: "userTotal", label: "用户总数", href: "/admin/users" },
  { key: "activeSystemTotal", label: "启用系统数", href: "/admin/systems" },
  { key: "portalManagedSystemTotal", label: "门户托管认证系统数", href: "/admin/integrations" },
  { key: "subsystemEntry24h", label: "24h 进入子系统次数", href: "/admin/audits" },
  { key: "highRiskPermissionChanges24h", label: "24h 高风险权限变更", href: "/admin/audits" },
];

export default function AdminOverviewPage() {
  const [data, setData] = useState<OverviewData | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    fetchApi<OverviewData>("/api/admin/overview")
      .then(setData)
      .catch((err: unknown) => {
        const { message } = handleApiError(err);
        setError(message);
      });
  }, []);

  if (error) return <p className="text-color-error">{error}</p>;
  if (!data) return <p className="text-text-muted">加载中...</p>;

  return (
    <div className="space-y-8">
      <h1 className="text-xl font-semibold text-text-primary">管理后台概览</h1>

      <section className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-5">
        {statCards.map((card) => (
          <Link
            key={card.key}
            href={card.href}
            className="rounded-radius-md border border-border-subtle bg-bg-secondary p-5 hover:border-border-muted"
          >
            <p className="text-xs uppercase tracking-wide text-text-muted">{card.label}</p>
            <p className="mt-2 text-3xl font-semibold text-text-primary">
              {(data.stats as any)[card.key]}
            </p>
          </Link>
        ))}
      </section>

      <section>
        <h2 className="mb-3 text-sm font-medium uppercase tracking-wide text-text-muted">待处理事项</h2>
        {data.todos.length === 0 ? (
          <p className="text-sm text-text-muted">暂无待处理事项</p>
        ) : (
          <div className="divide-y divide-border-faint rounded-radius-md border border-border-subtle bg-bg-secondary">
            {data.todos.map((todo) => (
              <div key={todo.title} className="px-5 py-3">
                <p className="text-sm text-text-secondary">{todo.title}</p>
              </div>
            ))}
          </div>
        )}
      </section>

      <section>
        <h2 className="mb-3 text-sm font-medium uppercase tracking-wide text-text-muted">最近审计</h2>
        <div className="divide-y divide-border-faint rounded-radius-md border border-border-subtle bg-bg-secondary">
          {data.recentAudits.length === 0 && (
            <p className="px-5 py-4 text-sm text-text-muted">暂无记录</p>
          )}
          {data.recentAudits.map((a) => (
            <div key={a.id} className="flex items-center justify-between px-5 py-3">
              <div>
                <p className="text-sm text-text-primary">
                  {a.actorName} · {a.action}
                </p>
                <p className="text-xs text-text-muted">
                  {a.targetType} {a.targetId}
                </p>
              </div>
              <span
                className={`text-xs ${a.result === "success" ? "text-color-success" : "text-color-error"}`}
              >
                {a.result === "success" ? "成功" : "失败"}
              </span>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
