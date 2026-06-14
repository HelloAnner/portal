"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Drawer } from "@/components/admin/drawer";
import { SelectField } from "@/components/admin/select-field";
import { Textarea } from "@/components/admin/textarea";
import { fetchApi, handleApiError, isAuthError, isPermissionError } from "@/lib/api-client";

interface Grant {
  id: string;
  userId: string;
  username: string;
  displayName: string;
  tenantId: string | null;
  tenantName: string | null;
  systemId: string;
  systemCode: string;
  systemName: string;
  adminType: string;
  scopes: Array<{ scopeType: string; scopeCode: string }>;
  status: string;
  reason?: string;
  startsAt: string;
  expiresAt: string | null;
}

interface System {
  id: string;
  code: string;
  name: string;
}

interface Tenant {
  id: string;
  code: string;
  name: string;
}

interface UserOption {
  id: string;
  username: string;
  displayName: string;
}

interface ScopeOptionsResponse {
  scopes: Array<{ scopeType: string; scopeCode: string; name?: string }>;
}

const adminTypes = [
  { value: "system", label: "系统级" },
  { value: "tenant", label: "租户级" },
  { value: "module", label: "模块级" },
  { value: "resource", label: "资源级" },
  { value: "organization", label: "组织级" },
];

const statuses = [
  { value: "", label: "全部状态" },
  { value: "active", label: "生效中" },
  { value: "inactive", label: "已撤销" },
  { value: "expired", label: "已过期" },
];

function normalizeList<T>(data: T[] | { items?: T[] } | unknown): T[] {
  if (Array.isArray(data)) return data as T[];
  if (data && typeof data === "object" && "items" in data) {
    return ((data as { items?: T[] }).items) || [];
  }
  return [];
}

export default function SubAdminsPage() {
  const [grants, setGrants] = useState<Grant[]>([]);
  const [systems, setSystems] = useState<System[]>([]);
  const [tenants, setTenants] = useState<Tenant[]>([]);
  const [users, setUsers] = useState<UserOption[]>([]);
  const [filters, setFilters] = useState({ status: "", systemCode: "", keyword: "" });
  const [loading, setLoading] = useState(true);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editing, setEditing] = useState<Grant | null>(null);
  const [error, setError] = useState("");

  function fetchGrants() {
    setLoading(true);
    setError("");
    const params = new URLSearchParams();
    if (filters.status) params.set("status", filters.status);
    if (filters.systemCode) params.set("systemCode", filters.systemCode);
    if (filters.keyword) params.set("keyword", filters.keyword);
    fetchApi<Grant[]>(`/api/admin/sub-admins?${params.toString()}`)
      .then(setGrants)
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
      fetchApi<System[] | { items?: System[] }>("/api/admin/integrations"),
      fetchApi<Tenant[] | { items?: Tenant[] }>("/api/admin/tenants"),
      fetchApi<UserOption[] | { items?: UserOption[] }>("/api/admin/users"),
    ])
      .then(([sysData, tenData, userData]) => {
        const systemsList = normalizeList<System>(sysData);
        setSystems(systemsList.map((s) => ({ id: s.id, code: s.code, name: s.name })));
        setTenants(normalizeList<Tenant>(tenData));
        setUsers(normalizeList<UserOption>(userData));
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
    fetchGrants();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filters.status, filters.systemCode]);

  function searchKeyword(e: React.FormEvent) {
    e.preventDefault();
    fetchGrants();
  }

  function openCreate() {
    setEditing(null);
    setDrawerOpen(true);
  }

  function openEdit(g: Grant) {
    setEditing(g);
    setDrawerOpen(true);
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <h1 className="text-xl font-semibold text-text-primary">子系统管理员</h1>
        <Button onClick={openCreate}>新建授权</Button>
      </div>

      {error && <p className="text-color-error">{error}</p>}

      <div className="flex flex-wrap items-end gap-3">
        <div className="w-40">
          <SelectField
            label="状态"
            value={filters.status}
            onChange={(e) => setFilters({ ...filters, status: e.target.value })}
            options={statuses}
          />
        </div>
        <div className="w-48">
          <SelectField
            label="系统"
            value={filters.systemCode}
            onChange={(e) => setFilters({ ...filters, systemCode: e.target.value })}
            options={[{ value: "", label: "全部系统" }, ...systems.map((s) => ({ value: s.code, label: s.name }))]}
          />
        </div>
        <form onSubmit={searchKeyword} className="flex items-end gap-2">
          <Input
            placeholder="搜索用户"
            value={filters.keyword}
            onChange={(e) => setFilters({ ...filters, keyword: e.target.value })}
          />
          <Button type="submit" variant="secondary">
            搜索
          </Button>
        </form>
      </div>

      {loading ? (
        <p className="text-text-muted">加载中...</p>
      ) : (
        <div className="overflow-auto rounded-radius-md border border-border-subtle">
          <table className="w-full text-left text-sm">
            <thead className="bg-bg-tertiary text-text-secondary">
              <tr>
                <th className="px-3 py-2 font-medium">用户</th>
                <th className="px-3 py-2 font-medium">系统</th>
                <th className="px-3 py-2 font-medium">租户</th>
                <th className="px-3 py-2 font-medium">类型</th>
                <th className="px-3 py-2 font-medium">范围</th>
                <th className="px-3 py-2 font-medium">状态</th>
                <th className="px-3 py-2 font-medium">有效期</th>
                <th className="px-3 py-2 font-medium">操作</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border-subtle">
              {grants.length === 0 && (
                <tr>
                  <td colSpan={8} className="px-3 py-4 text-text-muted">
                    暂无数据
                  </td>
                </tr>
              )}
              {grants.map((g) => (
                <tr key={g.id} className="hover:bg-hover-bg">
                  <td className="px-3 py-2 text-text-primary">{g.displayName || g.username}</td>
                  <td className="px-3 py-2 text-text-secondary">{g.systemName}</td>
                  <td className="px-3 py-2 text-text-secondary">{g.tenantName || "-"}</td>
                  <td className="px-3 py-2 text-text-secondary">
                    {adminTypes.find((t) => t.value === g.adminType)?.label || g.adminType}
                  </td>
                  <td className="px-3 py-2">
                    <div className="flex flex-wrap gap-1">
                      {g.scopes.map((s, i) => (
                        <Badge key={i} variant="default">
                          {s.scopeCode}
                        </Badge>
                      ))}
                    </div>
                  </td>
                  <td className="px-3 py-2">
                    <Badge variant={g.status === "active" ? "success" : g.status === "expired" ? "warning" : "default"}>
                      {g.status}
                    </Badge>
                  </td>
                  <td className="px-3 py-2 text-text-secondary">
                    {g.expiresAt ? new Date(g.expiresAt).toLocaleDateString() : "永久"}
                  </td>
                  <td className="px-3 py-2">
                    <Button variant="ghost" onClick={() => openEdit(g)}>
                      编辑
                    </Button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {drawerOpen && (
        <GrantDrawer
          key={editing?.id || "new"}
          open
          onClose={() => setDrawerOpen(false)}
          editing={editing}
          systems={systems}
          tenants={tenants}
          users={users}
          onSaved={fetchGrants}
        />
      )}
    </div>
  );
}

function GrantDrawer({
  open,
  onClose,
  editing,
  systems,
  tenants,
  users,
  onSaved,
}: {
  open: boolean;
  onClose: () => void;
  editing: Grant | null;
  systems: System[];
  tenants: Tenant[];
  users: UserOption[];
  onSaved: () => void;
}) {
  const [systemId, setSystemId] = useState(editing?.systemId || systems[0]?.id || "");
  const [userId, setUserId] = useState(editing?.userId || users[0]?.id || "");
  const [tenantId, setTenantId] = useState(editing?.tenantId || "");
  const [adminType, setAdminType] = useState(editing?.adminType || "module");
  const [status, setStatus] = useState(editing?.status || "active");
  const [scopes, setScopes] = useState<Record<string, boolean>>(() =>
    Object.fromEntries((editing?.scopes || []).map((s) => [`${s.scopeType}:${s.scopeCode}`, true]))
  );
  const [expiresAt, setExpiresAt] = useState(editing?.expiresAt ? editing.expiresAt.slice(0, 16) : "");
  const [reason, setReason] = useState(editing?.reason || "");
  const [scopeOptions, setScopeOptions] = useState<Array<{ scopeType: string; scopeCode: string; name?: string }>>([]);
  const [revokeTickets, setRevokeTickets] = useState(false);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    if (!systemId) return;
    const system = systems.find((s) => s.id === systemId);
    if (!system) return;
    fetchApi<ScopeOptionsResponse>(`/api/admin/sub-admins/scope-options?systemCode=${system.code}`)
      .then((data) => setScopeOptions(data.scopes || []))
      .catch((err: unknown) => {
        const { message } = handleApiError(err);
        alert(message);
      });
  }, [systemId, systems]);

  async function submit() {
    setSubmitting(true);
    const selectedScopes = Object.entries(scopes)
      .filter(([, v]) => v)
      .map(([key]) => {
        const [scopeType, scopeCode] = key.split(":");
        return { scopeType, scopeCode };
      });

    try {
      if (editing) {
        await fetchApi<unknown>(`/api/admin/sub-admins/${editing.id}`, {
          method: "PATCH",
          body: JSON.stringify({
            adminType,
            scopes: selectedScopes,
            status,
            reason,
            expiresAt: expiresAt || null,
            revokeActiveTickets: revokeTickets,
          }),
        });
      } else {
        await fetchApi<unknown>("/api/admin/sub-admins", {
          method: "POST",
          body: JSON.stringify({
            userId,
            tenantId: tenantId || null,
            systemId,
            adminType,
            scopes: selectedScopes,
            status,
            reason,
            expiresAt: expiresAt || null,
          }),
        });
      }
      onSaved();
      onClose();
    } catch (e: unknown) {
      const { message } = handleApiError(e);
      alert(message);
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Drawer
      open={open}
      onClose={onClose}
      title={editing ? "编辑授权" : "新建授权"}
      footer={
        <div className="flex justify-end gap-2">
          <Button variant="secondary" onClick={onClose}>
            取消
          </Button>
          <Button onClick={submit} disabled={submitting}>
            保存
          </Button>
        </div>
      }
    >
      <div className="space-y-4">
        {!editing && (
          <SelectField
            label="用户"
            value={userId}
            onChange={(e) => setUserId(e.target.value)}
            options={users.map((u) => ({ value: u.id, label: `${u.displayName || u.username} (${u.username})` }))}
          />
        )}
        {!editing && (
          <SelectField
            label="系统"
            value={systemId}
            onChange={(e) => setSystemId(e.target.value)}
            options={systems.map((s) => ({ value: s.id, label: s.name }))}
          />
        )}
        <SelectField
          label="租户（可选）"
          value={tenantId}
          onChange={(e) => setTenantId(e.target.value)}
          options={[{ value: "", label: "无" }, ...tenants.map((t) => ({ value: t.id, label: t.name }))]}
        />
        <SelectField
          label="管理员类型"
          value={adminType}
          onChange={(e) => setAdminType(e.target.value)}
          options={adminTypes}
        />
        <SelectField
          label="状态"
          value={status}
          onChange={(e) => setStatus(e.target.value)}
          options={[
            { value: "active", label: "生效中" },
            { value: "inactive", label: "已撤销" },
            { value: "expired", label: "已过期" },
          ]}
        />
        <div>
          <label className="text-sm font-medium text-text-secondary">管理范围</label>
          <div className="mt-2 space-y-2">
            {scopeOptions.length === 0 && <p className="text-xs text-text-muted">该系统未配置范围选项</p>}
            {scopeOptions.map((s) => {
              const key = `${s.scopeType}:${s.scopeCode}`;
              return (
                <label key={key} className="flex items-center gap-2 text-sm text-text-primary">
                  <input
                    type="checkbox"
                    checked={!!scopes[key]}
                    onChange={(e) => setScopes({ ...scopes, [key]: e.target.checked })}
                  />
                  {s.name || s.scopeCode}
                </label>
              );
            })}
          </div>
        </div>
        <div>
          <label className="text-sm font-medium text-text-secondary">过期时间</label>
          <Input type="datetime-local" value={expiresAt} onChange={(e) => setExpiresAt(e.target.value)} />
        </div>
        <div>
          <label className="text-sm font-medium text-text-secondary">原因 / 备注</label>
          <Textarea value={reason} onChange={(e) => setReason(e.target.value)} />
        </div>
        {editing && (
          <label className="flex items-center gap-2 text-sm text-text-primary">
            <input
              type="checkbox"
              checked={revokeTickets}
              onChange={(e) => setRevokeTickets(e.target.checked)}
            />
            同时撤销该用户的有效子系统票据
          </label>
        )}
      </div>
    </Drawer>
  );
}
