"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Drawer } from "@/components/admin/drawer";
import { SelectField } from "@/components/admin/select-field";
import { fetchApi, handleApiError, isAuthError, isPermissionError } from "@/lib/api-client";

interface Audit {
  id: string;
  requestId?: string;
  occurredAt: string;
  actorUserId?: string;
  actorName: string;
  action: string;
  targetType: string;
  targetId?: string;
  systemId?: string;
  systemName?: string;
  tenantId?: string;
  tenantName?: string;
  result: "success" | "failure";
  failureReason?: string;
}

interface AuditDetail extends Audit {
  beforeData: unknown;
  afterData: unknown;
  ipAddress?: string;
  userAgent?: string;
}

interface AuditListResponse {
  list: Audit[];
  total: number;
}

interface SystemOption {
  id: string;
  name: string;
}

interface TenantOption {
  id: string;
  name: string;
}

interface IntegrationItem {
  id: string;
  name: string;
}

interface TenantItem {
  id: string;
  name: string;
}

const results = [
  { value: "", label: "全部结果" },
  { value: "success", label: "成功" },
  { value: "failure", label: "失败" },
];

function normalizeList<T>(data: T[] | { items?: T[] } | unknown): T[] {
  if (Array.isArray(data)) return data as T[];
  if (data && typeof data === "object" && "items" in data) {
    return ((data as { items?: T[] }).items) || [];
  }
  return [];
}

export default function AuditsPage() {
  const [audits, setAudits] = useState<Audit[]>([]);
  const [total, setTotal] = useState(0);
  const [skip, setSkip] = useState(0);
  const take = 20;
  const [systems, setSystems] = useState<SystemOption[]>([]);
  const [tenants, setTenants] = useState<TenantOption[]>([]);
  const [filters, setFilters] = useState({
    occurredAtFrom: "",
    occurredAtTo: "",
    actorUserId: "",
    action: "",
    targetType: "",
    systemId: "",
    tenantId: "",
    result: "",
  });
  const [loading, setLoading] = useState(true);
  const [detail, setDetail] = useState<AuditDetail | null>(null);
  const [error, setError] = useState("");

  function buildParams(newSkip = skip) {
    const params = new URLSearchParams();
    params.set("skip", String(newSkip));
    params.set("take", String(take));
    if (filters.occurredAtFrom) params.set("occurredAtFrom", filters.occurredAtFrom);
    if (filters.occurredAtTo) params.set("occurredAtTo", filters.occurredAtTo);
    if (filters.actorUserId) params.set("actorUserId", filters.actorUserId);
    if (filters.action) params.set("action", filters.action);
    if (filters.targetType) params.set("targetType", filters.targetType);
    if (filters.systemId) params.set("systemId", filters.systemId);
    if (filters.tenantId) params.set("tenantId", filters.tenantId);
    if (filters.result) params.set("result", filters.result);
    return params;
  }

  function fetchAudits(newSkip = skip) {
    setLoading(true);
    setError("");
    fetchApi<AuditListResponse>(`/api/admin/audits?${buildParams(newSkip).toString()}`)
      .then((data) => {
        setAudits(data.list);
        setTotal(data.total);
      })
      .catch((err: unknown) => {
        if (isAuthError(err)) {
          window.location.href = "/login";
          return;
        }
        if (isPermissionError(err)) {
          window.location.href = "/no-permission";
          return;
        }
        const { message } = handleApiError(err);
        setError(message);
      })
      .finally(() => setLoading(false));
  }

  useEffect(() => {
    Promise.all([
      fetchApi<IntegrationItem[] | { items?: IntegrationItem[] }>("/api/admin/integrations"),
      fetchApi<TenantItem[] | { items?: TenantItem[] }>("/api/admin/tenants"),
    ])
      .then(([systemsData, tenantsData]) => {
        setSystems(
          normalizeList<IntegrationItem>(systemsData).map((s) => ({ id: s.id, name: s.name }))
        );
        setTenants(normalizeList<TenantItem>(tenantsData));
      })
      .catch((err: unknown) => {
        if (isAuthError(err)) {
          window.location.href = "/login";
          return;
        }
        if (isPermissionError(err)) {
          window.location.href = "/no-permission";
          return;
        }
        const { message } = handleApiError(err);
        setError(message);
      });
  }, []);

  useEffect(() => {
    fetchAudits(0);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function applyFilters(e: React.FormEvent) {
    e.preventDefault();
    setSkip(0);
    fetchAudits(0);
  }

  function nextPage() {
    const newSkip = skip + take;
    if (newSkip < total) {
      setSkip(newSkip);
      fetchAudits(newSkip);
    }
  }

  function prevPage() {
    const newSkip = Math.max(0, skip - take);
    setSkip(newSkip);
    fetchAudits(newSkip);
  }

  function openDetail(id: string) {
    fetchApi<AuditDetail>(`/api/admin/audits/${id}`)
      .then(setDetail)
      .catch((err: unknown) => {
        const { message } = handleApiError(err);
        alert(message);
      });
  }

  async function exportCsv() {
    const res = await fetch("/api/admin/audits/export", {
      method: "POST",
      credentials: "include",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        occurredAtFrom: filters.occurredAtFrom || undefined,
        occurredAtTo: filters.occurredAtTo || undefined,
        actorUserId: filters.actorUserId || undefined,
        action: filters.action || undefined,
        targetType: filters.targetType || undefined,
        systemId: filters.systemId || undefined,
        tenantId: filters.tenantId || undefined,
        result: filters.result || undefined,
      }),
    });
    if (!res.ok) {
      alert("导出失败");
      return;
    }
    const blob = await res.blob();
    const url = window.URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `audits_${Date.now()}.csv`;
    a.click();
    window.URL.revokeObjectURL(url);
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <h1 className="text-xl font-semibold text-text-primary">安全审计</h1>
        <Button variant="secondary" onClick={exportCsv}>
          导出 CSV
        </Button>
      </div>

      {error && <p className="text-color-error">{error}</p>}

      <form onSubmit={applyFilters} className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <Input
          type="datetime-local"
          value={filters.occurredAtFrom}
          onChange={(e) => setFilters({ ...filters, occurredAtFrom: e.target.value })}
        />
        <Input
          type="datetime-local"
          value={filters.occurredAtTo}
          onChange={(e) => setFilters({ ...filters, occurredAtTo: e.target.value })}
        />
        <Input placeholder="操作人 ID" value={filters.actorUserId} onChange={(e) => setFilters({ ...filters, actorUserId: e.target.value })} />
        <Input placeholder="操作类型" value={filters.action} onChange={(e) => setFilters({ ...filters, action: e.target.value })} />
        <Input placeholder="目标类型" value={filters.targetType} onChange={(e) => setFilters({ ...filters, targetType: e.target.value })} />
        <SelectField
          value={filters.systemId}
          onChange={(e) => setFilters({ ...filters, systemId: e.target.value })}
          options={[{ value: "", label: "全部系统" }, ...systems.map((s) => ({ value: s.id, label: s.name }))]}
        />
        <SelectField
          value={filters.tenantId}
          onChange={(e) => setFilters({ ...filters, tenantId: e.target.value })}
          options={[{ value: "", label: "全部租户" }, ...tenants.map((t) => ({ value: t.id, label: t.name }))]}
        />
        <SelectField
          value={filters.result}
          onChange={(e) => setFilters({ ...filters, result: e.target.value })}
          options={results}
        />
        <div className="sm:col-span-2 lg:col-span-4">
          <Button type="submit" variant="secondary">
            筛选
          </Button>
        </div>
      </form>

      {loading ? (
        <p className="text-text-muted">加载中...</p>
      ) : (
        <div className="overflow-auto rounded-radius-md border border-border-subtle">
          <table className="w-full text-left text-sm">
            <thead className="bg-bg-tertiary text-text-secondary">
              <tr>
                <th className="px-3 py-2 font-medium">时间</th>
                <th className="px-3 py-2 font-medium">操作人</th>
                <th className="px-3 py-2 font-medium">操作</th>
                <th className="px-3 py-2 font-medium">目标</th>
                <th className="px-3 py-2 font-medium">系统</th>
                <th className="px-3 py-2 font-medium">租户</th>
                <th className="px-3 py-2 font-medium">结果</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border-subtle">
              {audits.map((a) => (
                <tr key={a.id} className="hover:bg-hover-bg cursor-pointer" onClick={() => openDetail(a.id)}>
                  <td className="px-3 py-2 text-text-secondary">{new Date(a.occurredAt).toLocaleString()}</td>
                  <td className="px-3 py-2 text-text-primary">{a.actorName}</td>
                  <td className="px-3 py-2 text-text-secondary">{a.action}</td>
                  <td className="px-3 py-2 text-text-secondary">
                    {a.targetType} {a.targetId}
                  </td>
                  <td className="px-3 py-2 text-text-secondary">{a.systemName || "-"}</td>
                  <td className="px-3 py-2 text-text-secondary">{a.tenantName || "-"}</td>
                  <td className="px-3 py-2">
                    <Badge variant={a.result === "success" ? "success" : "error"}>{a.result}</Badge>
                  </td>
                </tr>
              ))}
              {audits.length === 0 && (
                <tr>
                  <td colSpan={7} className="px-3 py-4 text-text-muted">
                    暂无记录
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      )}

      <div className="flex items-center justify-between text-sm text-text-secondary">
        <span>共 {total} 条</span>
        <div className="flex gap-2">
          <Button variant="secondary" onClick={prevPage} disabled={skip === 0}>
            上一页
          </Button>
          <Button variant="secondary" onClick={nextPage} disabled={skip + take >= total}>
            下一页
          </Button>
        </div>
      </div>

      <Drawer
        open={!!detail}
        onClose={() => setDetail(null)}
        title="审计详情"
        footer={<Button variant="secondary" onClick={() => setDetail(null)}>关闭</Button>}
      >
        {detail && (
          <div className="space-y-4 text-sm">
            <div className="grid grid-cols-2 gap-3 text-text-secondary">
              <div>操作人：{detail.actorName}</div>
              <div>操作：{detail.action}</div>
              <div>
                目标：{detail.targetType} {detail.targetId}
              </div>
              <div>时间：{new Date(detail.occurredAt).toLocaleString()}</div>
              <div>请求ID：{detail.requestId || "-"}</div>
              <div>
                结果：
                <Badge variant={detail.result === "success" ? "success" : "error"}>{detail.result}</Badge>
              </div>
              {detail.systemName && <div>系统：{detail.systemName}</div>}
              {detail.tenantName && <div>租户：{detail.tenantName}</div>}
              {detail.ipAddress && <div>IP：{detail.ipAddress}</div>}
            </div>
            {detail.failureReason && (
              <div className="rounded-radius-sm bg-color-error/5 p-3 text-color-error">失败原因：{detail.failureReason}</div>
            )}
            <div>
              <h3 className="mb-1 font-medium text-text-primary">变更前</h3>
              <pre className="rounded-radius-sm bg-bg-tertiary p-3 text-xs text-text-secondary overflow-auto">
                {JSON.stringify(detail.beforeData, null, 2)}
              </pre>
            </div>
            <div>
              <h3 className="mb-1 font-medium text-text-primary">变更后</h3>
              <pre className="rounded-radius-sm bg-bg-tertiary p-3 text-xs text-text-secondary overflow-auto">
                {JSON.stringify(detail.afterData, null, 2)}
              </pre>
            </div>
          </div>
        )}
      </Drawer>
    </div>
  );
}
