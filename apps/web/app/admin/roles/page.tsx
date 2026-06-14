"use client";

import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Drawer } from "@/components/admin/drawer";
import { ChevronLeft, ChevronRight, Plus, Search, Trash2, X } from "lucide-react";
import { fetchApi, handleApiError, isAuthError, isPermissionError } from "@/lib/api-client";

interface RoleRow {
  id: string;
  code: string;
  name: string;
  roleType: string;
  description: string | null;
  isBuiltin: boolean;
  memberCount: number;
  createdAt: string;
}

interface RoleListResponse {
  items: RoleRow[];
  pagination: { total: number };
}

interface UserOption {
  id: string;
  username: string;
  displayName: string;
}

interface SystemOption {
  id: string;
  name: string;
  supportedPermissions?: string[];
}

interface TenantOption {
  id: string;
  code: string;
  name: string;
}

interface RoleDetail {
  role: {
    id: string;
    code: string;
    name: string;
    description: string | null;
    roleType: string;
    isBuiltin: boolean;
    userRoles: { user: UserOption }[];
  };
  permissionAssignments: Array<{
    id?: string;
    tenantId: string | null;
    systemId: string;
    visible: boolean;
    accessible: boolean;
    systemRoles: string[];
    permissions: string[];
    scopes: Array<{ scopeType: string; scopeCode: string }>;
    sourceNote: string | null;
    startsAt: string | null;
    expiresAt: string | null;
    system?: SystemOption | null;
    tenant?: TenantOption | null;
  }>;
  auditEvents: any[];
}

const roleTypeOptions = [
  { value: "", label: "全部类型" },
  { value: "super_admin", label: "超级管理员" },
  { value: "normal", label: "普通用户" },
  { value: "subsystem_admin", label: "子系统管理员" },
  { value: "custom", label: "自定义" },
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

export default function RolesPage() {
  const [items, setItems] = useState<RoleRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [page, setPage] = useState(1);
  const [pageSize] = useState(15);
  const [total, setTotal] = useState(0);

  const [keyword, setKeyword] = useState("");
  const [roleType, setRoleType] = useState("");

  const [detail, setDetail] = useState<RoleDetail | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<"info" | "members" | "permissions" | "audit">("info");

  const [users, setUsers] = useState<UserOption[]>([]);
  const [systems, setSystems] = useState<SystemOption[]>([]);
  const [tenants, setTenants] = useState<TenantOption[]>([]);

  const [createOpen, setCreateOpen] = useState(false);
  const [createForm, setCreateForm] = useState({ code: "", name: "", description: "", roleType: "custom" });
  const [saving, setSaving] = useState(false);

  const totalPages = useMemo(() => Math.max(1, Math.ceil(total / pageSize)), [total, pageSize]);

  async function fetchOptions() {
    try {
      const [u, s, t] = await Promise.all([
        fetchApi<UserOption[] | { items?: UserOption[] }>("/api/admin/users?pageSize=1000"),
        fetchApi<SystemOption[] | { items?: SystemOption[] }>("/api/admin/systems?pageSize=100"),
        fetchApi<TenantOption[] | { items?: TenantOption[] }>("/api/admin/tenants?pageSize=100"),
      ]);
      setUsers(normalizeList(u));
      setSystems(normalizeList(s));
      setTenants(normalizeList(t));
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    }
  }

  async function fetchRoles(targetPage = page) {
    setLoading(true);
    const params = new URLSearchParams();
    params.set("page", String(targetPage));
    params.set("pageSize", String(pageSize));
    if (keyword) params.set("keyword", keyword);
    if (roleType) params.set("roleType", roleType);
    try {
      const data = await fetchApi<RoleListResponse>(`/api/admin/roles?${params.toString()}`);
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
    fetchRoles(page);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [page, keyword, roleType]);

  async function openDetail(role: { id: string }) {
    try {
      const data = await fetchApi<RoleDetail>(`/api/admin/roles/${role.id}`);
      setDetail(data);
      setActiveTab("info");
      setDrawerOpen(true);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    }
  }

  async function saveInfo() {
    if (!detail) return;
    try {
      await fetchApi<unknown>(`/api/admin/roles/${detail.role.id}`, {
        method: "PATCH",
        body: JSON.stringify({
          name: detail.role.name,
          description: detail.role.description,
          roleType: detail.role.roleType,
        }),
      });
      openDetail(detail.role);
      fetchRoles(page);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    }
  }

  async function saveMembers() {
    if (!detail) return;
    try {
      const userIds = detail.role.userRoles.map((ur) => ur.user.id);
      await fetchApi<unknown>(`/api/admin/roles/${detail.role.id}/members`, {
        method: "PUT",
        body: JSON.stringify({ userIds }),
      });
      openDetail(detail.role);
      fetchRoles(page);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    }
  }

  async function savePermissions() {
    if (!detail) return;
    try {
      const assignments = detail.permissionAssignments.map((a) => ({
        tenantId: a.tenantId,
        systemId: a.systemId,
        visible: a.visible,
        accessible: a.accessible,
        systemRoles: a.systemRoles,
        permissions: a.permissions,
        scopes: a.scopes,
        sourceNote: a.sourceNote,
        startsAt: a.startsAt,
        expiresAt: a.expiresAt,
      }));
      await fetchApi<unknown>(`/api/admin/roles/${detail.role.id}/permissions`, {
        method: "PUT",
        body: JSON.stringify({ assignments }),
      });
      openDetail(detail.role);
      fetchRoles(page);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    }
  }

  async function deleteRole() {
    if (!detail) return;
    if (!confirm("确定删除该角色吗？")) return;
    try {
      await fetchApi<unknown>(`/api/admin/roles/${detail.role.id}`, { method: "DELETE" });
      setDrawerOpen(false);
      fetchRoles(page);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    }
  }

  async function createRole() {
    setSaving(true);
    try {
      await fetchApi<unknown>("/api/admin/roles", {
        method: "POST",
        body: JSON.stringify(createForm),
      });
      setCreateOpen(false);
      setCreateForm({ code: "", name: "", description: "", roleType: "custom" });
      fetchRoles(1);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    } finally {
      setSaving(false);
    }
  }

  function roleTypeLabel(type: string) {
    return roleTypeOptions.find((r) => r.value === type)?.label || type;
  }

  function updateAssignment(index: number, patch: Partial<RoleDetail["permissionAssignments"][0]>) {
    if (!detail) return;
    const next = [...detail.permissionAssignments];
    next[index] = { ...next[index], ...patch };
    setDetail({ ...detail, permissionAssignments: next });
  }

  function addAssignment() {
    if (!detail) return;
    const newAssignment: RoleDetail["permissionAssignments"][0] = {
      id: `tmp_${Date.now()}`,
      tenantId: null,
      systemId: systems[0]?.id || "",
      visible: true,
      accessible: true,
      systemRoles: [],
      permissions: [],
      scopes: [],
      sourceNote: null,
      startsAt: null,
      expiresAt: null,
      system: systems[0] || null,
      tenant: null,
    };
    setDetail({
      ...detail,
      permissionAssignments: [...detail.permissionAssignments, newAssignment],
    });
  }

  function removeAssignment(index: number) {
    if (!detail) return;
    const next = [...detail.permissionAssignments];
    next.splice(index, 1);
    setDetail({ ...detail, permissionAssignments: next });
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold text-text-primary">角色管理</h1>
        <Button onClick={() => setCreateOpen(true)}>
          <Plus className="h-4 w-4" />
          新建角色
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
          <label className="text-xs text-text-muted">类型</label>
          <select
            value={roleType}
            onChange={(e) => setRoleType(e.target.value)}
            className="h-10 rounded-radius-sm border border-border-subtle bg-bg-secondary px-3 text-sm text-text-primary"
          >
            {roleTypeOptions.map((r) => (
              <option key={r.value} value={r.value}>
                {r.label}
              </option>
            ))}
          </select>
        </div>
        <Button variant="secondary" onClick={() => fetchRoles(1)}>
          <Search className="h-4 w-4" />
          搜索
        </Button>
        <Button variant="ghost" onClick={() => { setKeyword(""); setRoleType(""); setPage(1); }}>
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
              <th className="px-4 py-3">类型</th>
              <th className="px-4 py-3">成员数</th>
              <th className="px-4 py-3">内置</th>
              <th className="px-4 py-3 text-right">操作</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border-faint">
            {items.map((r) => (
              <tr key={r.id} className="hover:bg-hover-bg">
                <td className="px-4 py-3 font-medium text-text-primary">{r.code}</td>
                <td className="px-4 py-3 text-text-secondary">{r.name}</td>
                <td className="px-4 py-3 text-text-secondary">{roleTypeLabel(r.roleType)}</td>
                <td className="px-4 py-3 text-text-secondary">{r.memberCount}</td>
                <td className="px-4 py-3">{r.isBuiltin ? <Badge variant="info">内置</Badge> : "-"}</td>
                <td className="px-4 py-3 text-right">
                  <Button variant="ghost" onClick={() => openDetail(r)}>
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
        title={detail?.role?.name || "角色详情"}
        width="640px"
      >
        {detail && (
          <div className="space-y-5">
            <div className="flex gap-2 border-b border-border-subtle pb-2">
              {([
                { key: "info", label: "基础信息" },
                { key: "members", label: "成员" },
                { key: "permissions", label: "权限矩阵" },
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
                    <div className="text-sm text-text-primary">{detail.role.code}</div>
                  </div>
                  <div>
                    <label className="text-xs text-text-muted">类型</label>
                    <div className="text-sm text-text-primary">{roleTypeLabel(detail.role.roleType)}</div>
                  </div>
                </div>
                <div>
                  <label className="text-xs text-text-muted">名称</label>
                  <Input
                    value={detail.role.name}
                    onChange={(e) => setDetail({ ...detail, role: { ...detail.role, name: e.target.value } })}
                  />
                </div>
                <div>
                  <label className="text-xs text-text-muted">描述</label>
                  <Input
                    value={detail.role.description || ""}
                    onChange={(e) => setDetail({ ...detail, role: { ...detail.role, description: e.target.value } })}
                  />
                </div>
                <div>
                  <label className="text-xs text-text-muted">类型</label>
                  <select
                    value={detail.role.roleType}
                    onChange={(e) => setDetail({ ...detail, role: { ...detail.role, roleType: e.target.value } })}
                    className="h-10 w-full rounded-radius-sm border border-border-subtle bg-bg-secondary px-3 text-sm text-text-primary"
                  >
                    {roleTypeOptions.filter((r) => r.value).map((r) => (
                      <option key={r.value} value={r.value}>
                        {r.label}
                      </option>
                    ))}
                  </select>
                </div>
                <div className="flex gap-2">
                  <Button onClick={saveInfo}>保存</Button>
                  {!detail.role.isBuiltin && (
                    <Button variant="danger" onClick={deleteRole}>
                      <Trash2 className="h-4 w-4" />
                      删除
                    </Button>
                  )}
                </div>
              </div>
            )}

            {activeTab === "members" && (
              <div className="space-y-3">
                <div className="rounded-radius-sm border border-border-subtle p-3">
                  {users.map((u) => {
                    const checked = detail.role.userRoles.some((ur) => ur.user.id === u.id);
                    return (
                      <label key={u.id} className="flex items-center gap-2 py-1 text-sm text-text-primary">
                        <input
                          type="checkbox"
                          checked={checked}
                          onChange={(e) => {
                            const next = e.target.checked
                              ? [...detail.role.userRoles, { user: u }]
                              : detail.role.userRoles.filter((ur) => ur.user.id !== u.id);
                            setDetail({ ...detail, role: { ...detail.role, userRoles: next } });
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

            {activeTab === "permissions" && (
              <div className="space-y-3">
                {detail.permissionAssignments.map((a, idx) => {
                  const system = systems.find((s) => s.id === a.systemId) || a.system;
                  const supportedPermissions = system?.supportedPermissions || [];
                  return (
                    <div key={a.id || idx} className="rounded-radius-sm border border-border-subtle p-3 space-y-3">
                      <div className="flex items-center gap-2">
                        <select
                          value={a.systemId}
                          onChange={(e) => {
                            const systemId = e.target.value;
                            const sys = systems.find((s) => s.id === systemId);
                            updateAssignment(idx, { systemId, system: sys, permissions: [] });
                          }}
                          className="h-9 flex-1 rounded-radius-sm border border-border-subtle bg-bg-secondary px-2 text-sm text-text-primary"
                        >
                          {systems.map((s) => (
                            <option key={s.id} value={s.id}>
                              {s.name}
                            </option>
                          ))}
                        </select>
                        <select
                          value={a.tenantId || ""}
                          onChange={(e) => {
                            const tenantId = e.target.value || null;
                            const tenant = tenants.find((t) => t.id === tenantId) || null;
                            updateAssignment(idx, { tenantId, tenant });
                          }}
                          className="h-9 flex-1 rounded-radius-sm border border-border-subtle bg-bg-secondary px-2 text-sm text-text-primary"
                        >
                          <option value="">全局/无租户</option>
                          {tenants.map((t) => (
                            <option key={t.id} value={t.id}>
                              {t.name}
                            </option>
                          ))}
                        </select>
                        <Button variant="ghost" onClick={() => removeAssignment(idx)}>
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                      <div className="flex gap-4 text-sm text-text-primary">
                        <label className="flex items-center gap-1">
                          <input
                            type="checkbox"
                            checked={a.visible}
                            onChange={(e) => updateAssignment(idx, { visible: e.target.checked })}
                          />
                          可见
                        </label>
                        <label className="flex items-center gap-1">
                          <input
                            type="checkbox"
                            checked={a.accessible}
                            onChange={(e) => updateAssignment(idx, { accessible: e.target.checked })}
                          />
                          可访问
                        </label>
                      </div>
                      {supportedPermissions.length > 0 && (
                        <div>
                          <div className="text-xs text-text-muted mb-1">权限</div>
                          <div className="flex flex-wrap gap-2">
                            {supportedPermissions.map((p) => (
                              <label key={p} className="flex items-center gap-1 text-sm text-text-primary">
                                <input
                                  type="checkbox"
                                  checked={a.permissions.includes(p)}
                                  onChange={(e) => {
                                    const next = e.target.checked
                                      ? [...a.permissions, p]
                                      : a.permissions.filter((x) => x !== p);
                                    updateAssignment(idx, { permissions: next });
                                  }}
                                />
                                {p}
                              </label>
                            ))}
                          </div>
                        </div>
                      )}
                      <div>
                        <div className="text-xs text-text-muted mb-1">系统角色（逗号分隔）</div>
                        <Input
                          value={a.systemRoles.join(",")}
                          onChange={(e) => updateAssignment(idx, { systemRoles: e.target.value.split(",").map((x) => x.trim()).filter(Boolean) })}
                        />
                      </div>
                    </div>
                  );
                })}
                <div className="flex gap-2">
                  <Button variant="secondary" onClick={addAssignment}>
                    <Plus className="h-4 w-4" />
                    添加权限行
                  </Button>
                  <Button onClick={savePermissions}>保存权限矩阵</Button>
                </div>
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
        title="新建角色"
        footer={
          <>
            <Button variant="secondary" onClick={() => setCreateOpen(false)}>
              取消
            </Button>
            <Button onClick={createRole} disabled={saving}>
              {saving ? "创建中..." : "创建"}
            </Button>
          </>
        }
      >
        <div className="space-y-5">
          <div className="space-y-2">
            <label className="text-sm text-text-secondary">角色编码</label>
            <Input value={createForm.code} onChange={(e) => setCreateForm({ ...createForm, code: e.target.value })} />
          </div>
          <div className="space-y-2">
            <label className="text-sm text-text-secondary">角色名称</label>
            <Input value={createForm.name} onChange={(e) => setCreateForm({ ...createForm, name: e.target.value })} />
          </div>
          <div className="space-y-2">
            <label className="text-sm text-text-secondary">描述</label>
            <Input value={createForm.description} onChange={(e) => setCreateForm({ ...createForm, description: e.target.value })} />
          </div>
          <div className="space-y-2">
            <label className="text-sm text-text-secondary">类型</label>
            <select
              value={createForm.roleType}
              onChange={(e) => setCreateForm({ ...createForm, roleType: e.target.value })}
              className="h-10 w-full rounded-radius-sm border border-border-subtle bg-bg-secondary px-3 text-sm text-text-primary"
            >
              {roleTypeOptions.filter((r) => r.value).map((r) => (
                <option key={r.value} value={r.value}>
                  {r.label}
                </option>
              ))}
            </select>
          </div>
        </div>
      </Drawer>
    </div>
  );
}
