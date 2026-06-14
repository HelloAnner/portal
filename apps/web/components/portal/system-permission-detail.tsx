"use client";

import { useEffect, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { fetchApi, handleApiError, isAuthError } from "@/lib/api-client";

interface AdminScope {
  scopeType: string;
  scopeCode: string;
  name?: string;
}

interface PermissionSource {
  type: string;
  label: string;
  visible: boolean;
  accessible: boolean;
  systemRoles: string[];
  permissions: string[];
  adminScopes: AdminScope[];
}

interface ContextDetail {
  tenantId: string;
  tenantName: string;
  identity: string;
  visible: boolean;
  accessible: boolean;
  permissions: string[];
  adminScopes: AdminScope[];
  sources: PermissionSource[];
  sourceSummary: string[];
  scopeSummary: string[];
}

interface SystemPermissionDetailData {
  systemCode: string;
  name: string;
  description: string | null;
  iconUrl: string | null;
  status: string;
  category: string | null;
  contexts: ContextDetail[];
  aggregate: {
    permissions: string[];
    adminScopes: AdminScope[];
    scopeSummary: string[];
    contextPreview: string;
  };
}

interface SystemPermissionDetailProps {
  systemCode: string;
}

function statusText(status: string): string {
  switch (status) {
    case "maintenance":
      return "维护中";
    case "onboarding":
      return "接入中";
    case "active":
      return "运行中";
    default:
      return status;
  }
}

function statusVariant(status: string): "success" | "warning" | "default" | "error" {
  switch (status) {
    case "active":
      return "success";
    case "maintenance":
      return "warning";
    case "onboarding":
      return "default";
    default:
      return "error";
  }
}

export function SystemPermissionDetail({ systemCode }: SystemPermissionDetailProps) {
  const [data, setData] = useState<SystemPermissionDetailData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  useEffect(() => {
    setLoading(true);
    setError("");
    fetchApi<SystemPermissionDetailData>(`/api/me/permissions/${systemCode}`)
      .then((data) => {
        setData(data);
        setLoading(false);
      })
      .catch((err: unknown) => {
        if (isAuthError(err)) {
          window.location.href = "/login";
          return;
        }
        const { message } = handleApiError(err);
        setError(message);
        setLoading(false);
      });
  }, [systemCode]);

  if (loading) return <p className="text-text-muted">加载中...</p>;
  if (error) return <p className="text-color-error">{error}</p>;
  if (!data) return null;

  return (
    <div className="space-y-6">
      <div>
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="text-lg font-semibold text-text-primary">{data.name}</h3>
            <p className="text-sm text-text-muted">{data.systemCode}</p>
          </div>
          <Badge variant={statusVariant(data.status)}>{statusText(data.status)}</Badge>
        </div>
        {data.description && <p className="mt-2 text-sm text-text-secondary">{data.description}</p>}
      </div>

      <section>
        <h4 className="mb-2 text-sm font-medium text-text-secondary">权限预览</h4>
        <p className="text-sm text-text-secondary">{data.aggregate.contextPreview || "暂无"}</p>
      </section>

      <section>
        <h4 className="mb-2 text-sm font-medium text-text-secondary">
          合并权限 ({data.aggregate.permissions.length})
        </h4>
        {data.aggregate.permissions.length === 0 ? (
          <p className="text-sm text-text-muted">无显式权限</p>
        ) : (
          <div className="flex flex-wrap gap-1.5">
            {data.aggregate.permissions.map((p) => (
              <Badge key={p} variant="default">
                {p}
              </Badge>
            ))}
          </div>
        )}
      </section>

      <section>
        <h4 className="mb-2 text-sm font-medium text-text-secondary">
          管理范围 ({data.aggregate.adminScopes.length})
        </h4>
        {data.aggregate.adminScopes.length === 0 ? (
          <p className="text-sm text-text-muted">无管理范围</p>
        ) : (
          <div className="flex flex-wrap gap-1.5">
            {data.aggregate.adminScopes.map((s) => (
              <Badge key={`${s.scopeType}:${s.scopeCode}`} variant="info">
                {s.name || `${s.scopeType}:${s.scopeCode}`}
              </Badge>
            ))}
          </div>
        )}
      </section>

      <section>
        <h4 className="mb-3 text-sm font-medium text-text-secondary">
          按租户明细 ({data.contexts.length})
        </h4>
        <div className="space-y-4">
          {data.contexts.map((ctx) => (
            <div
              key={ctx.tenantId}
              className="rounded-radius-md border border-border-subtle bg-bg-primary p-4"
            >
              <div className="mb-3 flex items-center justify-between">
                <span className="text-sm font-medium text-text-primary">{ctx.tenantName}</span>
                <div className="flex gap-1.5">
                  <Badge variant="identity">{ctx.identity}</Badge>
                  {ctx.accessible ? (
                    <Badge variant="success">可进入</Badge>
                  ) : (
                    <Badge variant="error">不可进入</Badge>
                  )}
                </div>
              </div>

              <div className="space-y-2">
                <div>
                  <p className="text-xs text-text-muted">来源</p>
                  <div className="mt-1 flex flex-wrap gap-1">
                    {ctx.sourceSummary.length === 0 ? (
                      <span className="text-xs text-text-muted">无</span>
                    ) : (
                      ctx.sourceSummary.map((s) => (
                        <Badge key={s} variant="default">
                          {s}
                        </Badge>
                      ))
                    )}
                  </div>
                </div>

                <div>
                  <p className="text-xs text-text-muted">范围</p>
                  <div className="mt-1 flex flex-wrap gap-1">
                    {ctx.scopeSummary.length === 0 ? (
                      <span className="text-xs text-text-muted">无</span>
                    ) : (
                      ctx.scopeSummary.map((s) => (
                        <Badge key={s} variant="info">
                          {s}
                        </Badge>
                      ))
                    )}
                  </div>
                </div>

                {ctx.permissions.length > 0 && (
                  <div>
                    <p className="text-xs text-text-muted">权限列表</p>
                    <div className="mt-1 flex flex-wrap gap-1">
                      {ctx.permissions.map((p) => (
                        <Badge key={p} variant="default">
                          {p}
                        </Badge>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
