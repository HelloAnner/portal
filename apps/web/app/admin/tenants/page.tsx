"use client";

import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Drawer } from "@/components/admin/drawer";
import { ChevronLeft, ChevronRight, Plus, Search, X } from "lucide-react";
import { fetchApi, handleApiError, isAuthError, isPermissionError } from "@/lib/api-client";

interface TenantRow {
  id: string;
  code: string;
  name: string;
  status: string;
  description: string | null;
  memberCount: number;
  systemCount: number;
  createdAt: string;
}

interface TenantListResponse {
  items: TenantRow[];
  pagination: { total: number };
}

interface UserOption {
  id: string;
  username: string;
  displayName: string;
}

interface SystemOption {
  id: string;
  code: string;
  name: string;
}

interface TenantDetail {
  tenant: {
    id: string;
    code: string;
    name: string;
    description: string | null;
    status: string;
    members: Array<{ user: UserOption; memberStatus: string }>;
    tenantSystems: Array<{ system: SystemOption; enabled: boolean }>;
  };
  auditEvents: any[];
  adminsBySystem: Record<string, Array<{ id: string; adminType: string; user: UserOption }>>;
}

const statusOptions = [
  { value: "", label: "全部状态" },
  { value: "active", label: "正常" },
  { value: "disabled", label: "已停用" },
  { value: "archived", label: "已归档" },
];

function normalizeList<T>(data: T[] | { items?: T[] } | unknown): T[] {
  if (Array.isArray(data)) return data as T[];
  if (data && typeof data === "object" && "items" in data) {
    return ((data as { items?: T[] }).items) || [];
  }
  return [];
}

function handleRedirect(err: unknown) {
  if (isAuthError(err)) {
    window.location.href = "/login";
    return true;
  }
  if (isPermissionError(err)) {
    window.location.href = "/no-permission";
    return true;
  }
  return false;
}

export default function TenantsPage() {
  const [items, setItems] = useState<TenantRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [page, setPage] = useState(1);
  const [pageSize] = useState(15);
  const [total, setTotal] = useState(0);

  const [keyword, setKeyword] = useState("");
  const [status, setStatus] = useState("");

  const [detail, setDetail] = useState<TenantDetail | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<"info" | "members" | "systems" | "audit">("info");

  const [users, setUsers] = useState<UserOption[]>([]);
  const [systems, setSystems] = useState<SystemOption[]>([]);

  const [createOpen, setCreateOpen] = useState(false);
  const [createForm, setCreateForm] = useState({ code: "", name: "", description: "", status: "active" });
  const [saving, setSaving] = useState(false);

  const totalPages = useMemo(() => Math.max(1, Math.ceil(total / pageSize)), [total, pageSize]);

  async function fetchOptions() {
    try {
      const [u, s] = await Promise.all([
        fetchApi<UserOption[] | { items?: UserOption[] }>("/api/admin/users?pageSize=1000"),
        fetchApi<SystemOption[] | { items?: SystemOption[] }>("/api/admin/systems?pageSize=100"),
      ]);
      setUsers(normalizeList(u));
      setSystems(normalizeList(s));
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    }
  }

  async function fetchTenants(targetPage = page) {
    setLoading(true);
    const params = new URLSearchParams();
    params.set("page", String(targetPage));
    params.set("pageSize", String(pageSize));
    if (keyword) params.set("keyword", keyword);
    if (status) params.set("status", status);
    try {
      const data = await fetchApi<TenantListResponse>(`/api/admin/tenants?${params.toString()}`);
      setItems(data.items ?? []);
      setTotal(data.pagination?.total ?? 0);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    fetchOptions();
  }, []);

  useEffect(() => {
    fetchTenants(page);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [page, keyword, status]);

  async function openDetail(tenant: { id: string }) {
    try {
      const data = await fetchApi<TenantDetail>(`/api/admin/tenants/${tenant.id}`);
      setDetail(data);
      setActiveTab("info");
      setDrawerOpen(true);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    }
  }

  async function saveMembers() {
    if (!detail) return;
    try {
      const userIds = detail.tenant.members.map((m) => m.user.id);
      await fetchApi<unknown>(`/api/admin/tenants/${detail.tenant.id}/members`, {
        method: "PUT",
        body: JSON.stringify({ userIds }),
      });
      openDetail(detail.tenant);
      fetchTenants(page);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    }
  }

  async function saveSystems() {
    if (!detail) return;
    try {
      const systemIds = detail.tenant.tenantSystems.map((ts) => ts.system.id);
      await fetchApi<unknown>(`/api/admin/tenants/${detail.tenant.id}/systems`, {
        method: "PUT",
        body: JSON.stringify({ systemIds }),
      });
      openDetail(detail.tenant);
      fetchTenants(page);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    }
  }

  async function saveTenantInfo(patch: Partial<TenantDetail["tenant"]>) {
    if (!detail) return;
    try {
      await fetchApi<unknown>(`/api/admin/tenants/${detail.tenant.id}`, {
        method: "PATCH",
        body: JSON.stringify(patch),
      });
      openDetail(detail.tenant);
      fetchTenants(page);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    }
  }

  async function createTenant() {
    setSaving(true);
    try {
      await fetchApi<unknown>("/api/admin/tenants", {
        method: "POST",
        body: JSON.stringify(createForm),
      });
      setCreateOpen(false);
      setCreateForm({ code: "", name: "", description: "", status: "active" });
      fetchTenants(1);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    } finally {
      setSaving(false);
    }
  }

  function statusBadge(status: string) {
    const variant =
      status === "active" ? "success" : status === "disabled" ? "error" : "default";
    const label = statusOptions.find((s) => s.value === status)?.label || status;
    return <Badge variant={variant as any}>{label}</Badge>;
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold text-text-primary">租户管理</h1>
        <Button onClick={() => setCreateOpen(true)}>
          <Plus className="h-4 w-4" />
          新建租户
        </Button>
      </div>

      <div className="flex flex-wrap items-end gap-3 rounded-radius-md border border-border-subtle bg-bg-secondary p-4">
        <div className="flex flex-col gap-1">
          <label className="text-xs text-text-muted">关键词</label>
          <Input
            placeholder="编码/名称"
            value={keyword}
            onChange={(e) => setKeyword(e.target.value)}
            className="w-64"
          />
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs text-text-muted">状态</label>
          <select
            value={status}
            onChange={(e) => setStatus(e.target.value)}
            className="h-10 rounded-radius-sm border border-border-subtle bg-bg-secondary px-3 text-sm text-text-primary"
          >
            {statusOptions.map((s) => (
              <option key={s.value} value={s.value}>
                {s.label}
              </option>
            ))}
          </select>
        </div>
        <Button variant="secondary" onClick={() => fetchTenants(1)}>
          <Search className="h-4 w-4" />
          搜索
        </Button>
        <Button variant="ghost" onClick={() => { setKeyword(""); setStatus(""); setPage(1); }}>
          <X className="h-4 w-4" />
          重置
        </Button>
      </div>

      <div className="overflow-hidden rounded-radius-md border border-border-subtle bg-bg-secondary">
        <table className="w-full text-left text-sm">
          <thead className="bg-bg-tertiary text-text-secondary">
            <tr>
              <th className="px-4 py-3">编码</th>
              <th className="px-4 py-3">名称</th>
              <th className="px-4 py-3">状态</th>
              <th className="px-4 py-3">成员数</th>
              <th className="px-4 py-3">系统数</th>
              <th className="px-4 py-3 text-right">操作</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border-faint">
            {items.map((t) => (
              <tr key={t.id} className="hover:bg-hover-bg">
                <td className="px-4 py-3 font-medium text-text-primary">{t.code}</td>
                <td className="px-4 py-3 text-text-secondary">{t.name}</td>
                <td className="px-4 py-3">{statusBadge(t.status)}</td>
                <td className="px-4 py-3 text-text-secondary">{t.memberCount}</td>
                <td className="px-4 py-3 text-text-secondary">{t.systemCount}</td>
                <td className="px-4 py-3 text-right">
                  <Button variant="ghost" onClick={() => openDetail(t)}>
                    详情
                  </Button>
                </td>
              </tr>
            ))}
            {items.length === 0 && !loading && (
              <tr>
                <td colSpan={6} className="px-4 py-8 text-center text-text-muted">
                  暂无数据
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <div className="flex items-center justify-between">
        <span className="text-sm text-text-muted">
          共 {total} 条，第 {page}/{totalPages} 页
        </span>
        <div className="flex items-center gap-2">
          <Button variant="secondary" disabled={page <= 1} onClick={() => setPage(page - 1)}>
            <ChevronLeft className="h-4 w-4" />
          </Button>
          <Button variant="secondary" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>
      </div>

      <Drawer
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        title={detail?.tenant?.name || "租户详情"}
        width="560px"
      >
        {detail && (
          <div className="space-y-5">
            <div className="flex gap-2 border-b border-border-subtle pb-2">
              {([
                { key: "info", label: "基础信息" },
                { key: "members", label: "成员" },
                { key: "systems", label: "启用系统" },
                { key: "audit", label: "审计" },
              ] as const).map((tab) => (
                <button
                  key={tab.key}
                  onClick={() => setActiveTab(tab.key)}
                  className={`px-3 py-1.5 text-sm rounded-radius-sm transition-colors ${
                    activeTab === tab.key
                      ? "bg-selected-bg text-text-primary"
                      : "text-text-secondary hover:bg-hover-bg"
                  }`}
                >
                  {tab.label}
                </button>
              ))}
            </div>

            {activeTab === "info" && (
              <div className="space-y-4">
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="text-xs text-text-muted">编码</label>
                    <div className="text-sm text-text-primary">{detail.tenant.code}</div>
                  </div>
                  <div>
                    <label className="text-xs text-text-muted">状态</label>
                    <div>{statusBadge(detail.tenant.status)}</div>
                  </div>
                </div>
                <div>
                  <label className="text-xs text-text-muted">名称</label>
                  <Input
                    value={detail.tenant.name}
                    onChange={(e) =>
                      setDetail({ ...detail, tenant: { ...detail.tenant, name: e.target.value } })
                    }
                  />
                </div>
                <div>
                  <label className="text-xs text-text-muted">描述</label>
                  <Input
                    value={detail.tenant.description || ""}
                    onChange={(e) =>
                      setDetail({ ...detail, tenant: { ...detail.tenant, description: e.target.value } })
                    }
                  />
                </div>
                <div>
                  <label className="text-xs text-text-muted">状态</label>
                  <select
                    value={detail.tenant.status}
                    onChange={(e) =>
                      setDetail({ ...detail, tenant: { ...detail.tenant, status: e.target.value } })
                    }
                    className="h-10 w-full rounded-radius-sm border border-border-subtle bg-bg-secondary px-3 text-sm text-text-primary"
                  >
                    {statusOptions.filter((s) => s.value).map((s) => (
                      <option key={s.value} value={s.value}>
                        {s.label}
                      </option>
                    ))}
                  </select>
                </div>
                <Button onClick={() => saveTenantInfo({ name: detail.tenant.name, description: detail.tenant.description, status: detail.tenant.status })}>
                  保存基础信息
                </Button>

                {Object.keys(detail.adminsBySystem).length > 0 && (
                  <div>
                    <label className="text-xs text-text-muted">各系统租户管理员</label>
                    <div className="mt-2 space-y-2">
                      {detail.tenant.tenantSystems.map((ts) => {
                        const admins = detail.adminsBySystem[ts.system.id] || [];
                        return (
                          <div key={ts.system.id} className="rounded-radius-sm border border-border-subtle p-3">
                            <div className="text-sm font-medium text-text-primary">{ts.system.name}</div>
                            <div className="mt-1 flex flex-wrap gap-1">
                              {admins.length === 0 && (
                                <span className="text-xs text-text-muted">暂无管理员</span>
                              )}
                              {admins.map((g) => (
                                <Badge key={g.id} variant="identity">
                                  {g.user.displayName} ({g.adminType})
                                </Badge>
                              ))}
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                )}
              </div>
            )}

            {activeTab === "members" && (
              <div className="space-y-3">
                <div className="rounded-radius-sm border border-border-subtle p-3">
                  {users.map((u) => {
                    const checked = detail.tenant.members.some((m) => m.user.id === u.id);
                    return (
                      <label key={u.id} className="flex items-center gap-2 py-1 text-sm text-text-primary">
                        <input
                          type="checkbox"
                          checked={checked}
                          onChange={(e) => {
                            const next = e.target.checked
                              ? [...detail.tenant.members, { user: u, memberStatus: "active" }]
                              : detail.tenant.members.filter((m) => m.user.id !== u.id);
                            setDetail({ ...detail, tenant: { ...detail.tenant, members: next } });
                          }}
                        />
                        {u.displayName} ({u.username})
                      </label>
                    );
                  })}
                </div>
                <Button onClick={saveMembers}>保存成员</Button>
              </div>
            )}

            {activeTab === "systems" && (
              <div className="space-y-3">
                <div className="rounded-radius-sm border border-border-subtle p-3">
                  {systems.map((s) => {
                    const checked = detail.tenant.tenantSystems.some((ts) => ts.system.id === s.id);
                    return (
                      <label key={s.id} className="flex items-center gap-2 py-1 text-sm text-text-primary">
                        <input
                          type="checkbox"
                          checked={checked}
                          onChange={(e) => {
                            const next = e.target.checked
                              ? [...detail.tenant.tenantSystems, { system: s, enabled: true }]
                              : detail.tenant.tenantSystems.filter((ts) => ts.system.id !== s.id);
                            setDetail({ ...detail, tenant: { ...detail.tenant, tenantSystems: next } });
                          }}
                        />
                        {s.name} ({s.code})
                      </label>
                    );
                  })}
                </div>
                <Button onClick={saveSystems}>保存启用系统</Button>
              </div>
            )}

            {activeTab === "audit" && (
              <div className="space-y-2">
                {detail.auditEvents.map((a) => (
                  <div key={a.id} className="rounded-radius-sm border border-border-subtle p-3 text-sm">
                    <div className="flex items-center justify-between">
                      <span className="font-medium text-text-primary">{a.action}</span>
                      <Badge variant={a.result === "success" ? "success" : "error"}>{a.result === "success" ? "成功" : "失败"}</Badge>
                    </div>
                    <div className="text-xs text-text-muted">
                      {new Date(a.occurredAt).toLocaleString()} · {a.actor?.displayName || "系统"}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </Drawer>

      <Drawer
        open={createOpen}
        onClose={() => setCreateOpen(false)}
        title="新建租户"
        footer={
          <>
            <Button variant="secondary" onClick={() => setCreateOpen(false)}>
              取消
            </Button>
            <Button onClick={createTenant} disabled={saving}>
              {saving ? "创建中..." : "创建"}
            </Button>
          </>
        }
      >
        <div className="space-y-5">
          <div className="space-y-2">
            <label className="text-sm text-text-secondary">租户编码</label>
            <Input
              value={createForm.code}
              onChange={(e) => setCreateForm({ ...createForm, code: e.target.value })}
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm text-text-secondary">租户名称</label>
            <Input
              value={createForm.name}
              onChange={(e) => setCreateForm({ ...createForm, name: e.target.value })}
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm text-text-secondary">描述</label>
            <Input
              value={createForm.description}
              onChange={(e) => setCreateForm({ ...createForm, description: e.target.value })}
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm text-text-secondary">状态</label>
            <select
              value={createForm.status}
              onChange={(e) => setCreateForm({ ...createForm, status: e.target.value })}
              className="h-10 w-full rounded-radius-sm border border-border-subtle bg-bg-secondary px-3 text-sm text-text-primary"
            >
              {statusOptions.filter((s) => s.value).map((s) => (
                <option key={s.value} value={s.value}>
                  {s.label}
                </option>
              ))}
            </select>
          </div>
        </div>
      </Drawer>
    </div>
  );
}
