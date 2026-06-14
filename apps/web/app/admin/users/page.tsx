"use client";

import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Drawer } from "@/components/admin/drawer";
import { Modal } from "@/components/admin/modal";
import { ChevronLeft, ChevronRight, Plus, Search, X } from "lucide-react";
import { cn } from "@/lib/utils";
import { fetchApi, handleApiError, isAuthError, isPermissionError } from "@/lib/api-client";

interface UserRow {
  id: string;
  username: string;
  displayName: string;
  email: string | null;
  phone: string | null;
  status: string;
  organizationPath: string | null;
  createdAt: string;
  userRoles: { role: { id: string; code: string; name: string } }[];
  tenantMembers: { tenant: { id: string; code: string; name: string } }[];
  sessionCount: number;
}

interface UserListResponse {
  items: UserRow[];
  pagination: { total: number };
}

interface Option {
  id: string;
  code?: string;
  name: string;
}

interface CreateUserResponse {
  id: string;
}

const statusOptions = [
  { value: "", label: "全部状态" },
  { value: "active", label: "正常" },
  { value: "disabled", label: "已禁用" },
  { value: "pending", label: "待激活" },
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

export default function UsersPage() {
  const [items, setItems] = useState<UserRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [page, setPage] = useState(1);
  const [pageSize] = useState(15);
  const [total, setTotal] = useState(0);

  const [keyword, setKeyword] = useState("");
  const [status, setStatus] = useState("");
  const [roleCode, setRoleCode] = useState("");
  const [tenantId, setTenantId] = useState("");
  const [systemCode, setSystemCode] = useState("");
  const [organizationPath, setOrganizationPath] = useState("");

  const [roles, setRoles] = useState<Option[]>([]);
  const [tenants, setTenants] = useState<Option[]>([]);
  const [systems, setSystems] = useState<Option[]>([]);

  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingUser, setEditingUser] = useState<UserRow | null>(null);

  const [form, setForm] = useState<any>({});
  const [formRoleIds, setFormRoleIds] = useState<string[]>([]);
  const [formTenantIds, setFormTenantIds] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);

  const [batchAction, setBatchAction] = useState("");
  const [batchRoleId, setBatchRoleId] = useState("");
  const [batchTenantId, setBatchTenantId] = useState("");

  const totalPages = useMemo(() => Math.max(1, Math.ceil(total / pageSize)), [total, pageSize]);

  async function fetchOptions() {
    try {
      const [r, t, s] = await Promise.all([
        fetchApi<Option[] | { items?: Option[] }>("/api/admin/roles?pageSize=100"),
        fetchApi<Option[] | { items?: Option[] }>("/api/admin/tenants?pageSize=100"),
        fetchApi<Option[] | { items?: Option[] }>("/api/admin/systems?pageSize=100"),
      ]);
      setRoles(normalizeList(r));
      setTenants(normalizeList(t));
      setSystems(normalizeList(s));
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    }
  }

  async function fetchUsers(targetPage = page) {
    setLoading(true);
    const params = new URLSearchParams();
    params.set("page", String(targetPage));
    params.set("pageSize", String(pageSize));
    if (keyword) params.set("keyword", keyword);
    if (status) params.set("status", status);
    if (roleCode) params.set("roleCode", roleCode);
    if (tenantId) params.set("tenantId", tenantId);
    if (systemCode) params.set("systemCode", systemCode);
    if (organizationPath) params.set("organizationPath", organizationPath);

    try {
      const data = await fetchApi<UserListResponse>(`/api/admin/users?${params.toString()}`);
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
    fetchUsers(page);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [page, keyword, status, roleCode, tenantId, systemCode, organizationPath]);

  function resetFilters() {
    setKeyword("");
    setStatus("");
    setRoleCode("");
    setTenantId("");
    setSystemCode("");
    setOrganizationPath("");
    setPage(1);
  }

  function openCreate() {
    setEditingUser(null);
    setForm({
      username: "",
      displayName: "",
      email: "",
      phone: "",
      organizationPath: "",
      status: "active",
      password: "",
    });
    setFormRoleIds([]);
    setFormTenantIds([]);
    setDrawerOpen(true);
  }

  function openEdit(user: UserRow) {
    setEditingUser(user);
    setForm({
      username: user.username,
      displayName: user.displayName,
      email: user.email ?? "",
      phone: user.phone ?? "",
      organizationPath: user.organizationPath ?? "",
      status: user.status,
    });
    setFormRoleIds(user.userRoles.map((ur) => ur.role.id));
    setFormTenantIds(user.tenantMembers.map((tm) => tm.tenant.id));
    setDrawerOpen(true);
  }

  async function saveUser() {
    setSaving(true);
    try {
      let userId = editingUser?.id;
      if (editingUser) {
        await fetchApi<unknown>(`/api/admin/users/${userId}`, {
          method: "PATCH",
          body: JSON.stringify(form),
        });
      } else {
        const data = await fetchApi<CreateUserResponse>("/api/admin/users", {
          method: "POST",
          body: JSON.stringify({ ...form, roleIds: formRoleIds, tenantIds: formTenantIds }),
        });
        userId = data.id;
      }

      if (userId) {
        await fetchApi<unknown>(`/api/admin/users/${userId}/roles`, {
          method: "PUT",
          body: JSON.stringify({ roleIds: formRoleIds }),
        });
        await fetchApi<unknown>(`/api/admin/users/${userId}/tenants`, {
          method: "PUT",
          body: JSON.stringify({ tenantIds: formTenantIds }),
        });
      }

      setDrawerOpen(false);
      fetchUsers(page);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message || "保存失败");
    } finally {
      setSaving(false);
    }
  }

  async function runBatch() {
    if (!batchAction || selectedIds.size === 0) return;
    const body: any = { action: batchAction, userIds: Array.from(selectedIds) };
    if (["assignRole", "removeRole"].includes(batchAction)) {
      if (!batchRoleId) return alert("请选择角色");
      body.roleId = batchRoleId;
    }
    if (["addToTenant", "removeFromTenant"].includes(batchAction)) {
      if (!batchTenantId) return alert("请选择租户");
      body.tenantId = batchTenantId;
    }

    try {
      await fetchApi<unknown>("/api/admin/users/batch", {
        method: "POST",
        body: JSON.stringify(body),
      });
      setSelectedIds(new Set());
      setBatchAction("");
      fetchUsers(page);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message || "批量操作失败");
    }
  }

  function toggleSelect(id: string) {
    const next = new Set(selectedIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setSelectedIds(next);
  }

  function toggleSelectAll() {
    if (selectedIds.size === items.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(items.map((i) => i.id)));
    }
  }

  function statusBadge(status: string) {
    const variant =
      status === "active"
        ? "success"
        : status === "disabled"
        ? "error"
        : status === "pending"
        ? "warning"
        : "default";
    const label = statusOptions.find((s) => s.value === status)?.label || status;
    return <Badge variant={variant as any}>{label}</Badge>;
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold text-text-primary">用户管理</h1>
        <Button onClick={openCreate}>
          <Plus className="h-4 w-4" />
          新建用户
        </Button>
      </div>

      <div className="flex flex-wrap items-end gap-3 rounded-radius-md border border-border-subtle bg-bg-secondary p-4">
        <div className="flex flex-col gap-1">
          <label className="text-xs text-text-muted">关键词</label>
          <Input
            placeholder="用户名/姓名/邮箱/手机"
            value={keyword}
            onChange={(e) => setKeyword(e.target.value)}
            className="w-52"
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
        <div className="flex flex-col gap-1">
          <label className="text-xs text-text-muted">角色</label>
          <select
            value={roleCode}
            onChange={(e) => setRoleCode(e.target.value)}
            className="h-10 rounded-radius-sm border border-border-subtle bg-bg-secondary px-3 text-sm text-text-primary"
          >
            <option value="">全部角色</option>
            {roles.map((r) => (
              <option key={r.id} value={r.code}>
                {r.name}
              </option>
            ))}
          </select>
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs text-text-muted">租户</label>
          <select
            value={tenantId}
            onChange={(e) => setTenantId(e.target.value)}
            className="h-10 rounded-radius-sm border border-border-subtle bg-bg-secondary px-3 text-sm text-text-primary"
          >
            <option value="">全部租户</option>
            {tenants.map((t) => (
              <option key={t.id} value={t.id}>
                {t.name}
              </option>
            ))}
          </select>
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs text-text-muted">系统</label>
          <select
            value={systemCode}
            onChange={(e) => setSystemCode(e.target.value)}
            className="h-10 rounded-radius-sm border border-border-subtle bg-bg-secondary px-3 text-sm text-text-primary"
          >
            <option value="">全部系统</option>
            {systems.map((s) => (
              <option key={s.id} value={s.code}>
                {s.name}
              </option>
            ))}
          </select>
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs text-text-muted">组织路径</label>
          <Input
            placeholder="组织路径"
            value={organizationPath}
            onChange={(e) => setOrganizationPath(e.target.value)}
            className="w-40"
          />
        </div>
        <Button variant="secondary" onClick={() => fetchUsers(1)}>
          <Search className="h-4 w-4" />
          搜索
        </Button>
        <Button variant="ghost" onClick={resetFilters}>
          <X className="h-4 w-4" />
          重置
        </Button>
      </div>

      {selectedIds.size > 0 && (
        <div className="flex items-center gap-3 rounded-radius-md border border-border-subtle bg-bg-secondary p-3">
          <span className="text-sm text-text-secondary">已选择 {selectedIds.size} 项</span>
          <select
            value={batchAction}
            onChange={(e) => setBatchAction(e.target.value)}
            className="h-9 rounded-radius-sm border border-border-subtle bg-bg-secondary px-3 text-sm text-text-primary"
          >
            <option value="">批量操作</option>
            <option value="enable">启用</option>
            <option value="disable">禁用</option>
            <option value="assignRole">分配角色</option>
            <option value="removeRole">移除角色</option>
            <option value="addToTenant">加入租户</option>
            <option value="removeFromTenant">移出租户</option>
          </select>
          {["assignRole", "removeRole"].includes(batchAction) && (
            <select
              value={batchRoleId}
              onChange={(e) => setBatchRoleId(e.target.value)}
              className="h-9 rounded-radius-sm border border-border-subtle bg-bg-secondary px-3 text-sm text-text-primary"
            >
              <option value="">选择角色</option>
              {roles.map((r) => (
                <option key={r.id} value={r.id}>
                  {r.name}
                </option>
              ))}
            </select>
          )}
          {["addToTenant", "removeFromTenant"].includes(batchAction) && (
            <select
              value={batchTenantId}
              onChange={(e) => setBatchTenantId(e.target.value)}
              className="h-9 rounded-radius-sm border border-border-subtle bg-bg-secondary px-3 text-sm text-text-primary"
            >
              <option value="">选择租户</option>
              {tenants.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.name}
                </option>
              ))}
            </select>
          )}
          <Button onClick={runBatch}>执行</Button>
        </div>
      )}

      <div className="overflow-hidden rounded-radius-md border border-border-subtle bg-bg-secondary">
        <table className="w-full text-left text-sm">
          <thead className="bg-bg-tertiary text-text-secondary">
            <tr>
              <th className="px-4 py-3">
                <input
                  type="checkbox"
                  checked={items.length > 0 && selectedIds.size === items.length}
                  onChange={toggleSelectAll}
                />
              </th>
              <th className="px-4 py-3">用户名</th>
              <th className="px-4 py-3">显示名</th>
              <th className="px-4 py-3">邮箱</th>
              <th className="px-4 py-3">状态</th>
              <th className="px-4 py-3">角色</th>
              <th className="px-4 py-3">租户</th>
              <th className="px-4 py-3">组织路径</th>
              <th className="px-4 py-3 text-right">操作</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border-faint">
            {items.map((user) => (
              <tr key={user.id} className="hover:bg-hover-bg">
                <td className="px-4 py-3">
                  <input
                    type="checkbox"
                    checked={selectedIds.has(user.id)}
                    onChange={() => toggleSelect(user.id)}
                  />
                </td>
                <td className="px-4 py-3 font-medium text-text-primary">{user.username}</td>
                <td className="px-4 py-3 text-text-secondary">{user.displayName}</td>
                <td className="px-4 py-3 text-text-secondary">{user.email || "-"}</td>
                <td className="px-4 py-3">{statusBadge(user.status)}</td>
                <td className="px-4 py-3">
                  <div className="flex flex-wrap gap-1">
                    {user.userRoles.map((ur) => (
                      <Badge key={ur.role.id} variant="identity">
                        {ur.role.name}
                      </Badge>
                    ))}
                  </div>
                </td>
                <td className="px-4 py-3">
                  <div className="flex flex-wrap gap-1">
                    {user.tenantMembers.map((tm) => (
                      <span key={tm.tenant.id} className="text-text-secondary">
                        {tm.tenant.name}
                      </span>
                    ))}
                  </div>
                </td>
                <td className="px-4 py-3 text-text-muted">{user.organizationPath || "-"}</td>
                <td className="px-4 py-3 text-right">
                  <Button variant="ghost" onClick={() => openEdit(user)}>
                    编辑
                  </Button>
                </td>
              </tr>
            ))}
            {items.length === 0 && !loading && (
              <tr>
                <td colSpan={9} className="px-4 py-8 text-center text-text-muted">
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
        title={editingUser ? "编辑用户" : "新建用户"}
        footer={
          <>
            <Button variant="secondary" onClick={() => setDrawerOpen(false)}>
              取消
            </Button>
            <Button onClick={saveUser} disabled={saving}>
              {saving ? "保存中..." : "保存"}
            </Button>
          </>
        }
      >
        <div className="space-y-5">
          <div className="space-y-2">
            <label className="text-sm text-text-secondary">用户名</label>
            <Input
              value={form.username || ""}
              onChange={(e) => setForm({ ...form, username: e.target.value })}
              disabled={!!editingUser}
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm text-text-secondary">显示名</label>
            <Input
              value={form.displayName || ""}
              onChange={(e) => setForm({ ...form, displayName: e.target.value })}
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm text-text-secondary">邮箱</label>
            <Input
              value={form.email || ""}
              onChange={(e) => setForm({ ...form, email: e.target.value })}
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm text-text-secondary">手机</label>
            <Input
              value={form.phone || ""}
              onChange={(e) => setForm({ ...form, phone: e.target.value })}
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm text-text-secondary">组织路径</label>
            <Input
              value={form.organizationPath || ""}
              onChange={(e) => setForm({ ...form, organizationPath: e.target.value })}
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm text-text-secondary">状态</label>
            <select
              value={form.status || "active"}
              onChange={(e) => setForm({ ...form, status: e.target.value })}
              className="h-10 w-full rounded-radius-sm border border-border-subtle bg-bg-secondary px-3 text-sm text-text-primary"
            >
              {statusOptions.filter((s) => s.value).map((s) => (
                <option key={s.value} value={s.value}>
                  {s.label}
                </option>
              ))}
            </select>
          </div>
          {!editingUser && (
            <div className="space-y-2">
              <label className="text-sm text-text-secondary">初始密码</label>
              <Input
                type="password"
                placeholder="留空使用 portal123"
                value={form.password || ""}
                onChange={(e) => setForm({ ...form, password: e.target.value })}
              />
            </div>
          )}
          <div className="space-y-2">
            <label className="text-sm text-text-secondary">门户角色</label>
            <div className="space-y-2 rounded-radius-sm border border-border-subtle p-3">
              {roles.map((r) => (
                <label key={r.id} className="flex items-center gap-2 text-sm text-text-primary">
                  <input
                    type="checkbox"
                    checked={formRoleIds.includes(r.id)}
                    onChange={(e) => {
                      if (e.target.checked) setFormRoleIds([...formRoleIds, r.id]);
                      else setFormRoleIds(formRoleIds.filter((id) => id !== r.id));
                    }}
                  />
                  {r.name}
                </label>
              ))}
            </div>
          </div>
          <div className="space-y-2">
            <label className="text-sm text-text-secondary">所属租户</label>
            <div className="space-y-2 rounded-radius-sm border border-border-subtle p-3">
              {tenants.map((t) => (
                <label key={t.id} className="flex items-center gap-2 text-sm text-text-primary">
                  <input
                    type="checkbox"
                    checked={formTenantIds.includes(t.id)}
                    onChange={(e) => {
                      if (e.target.checked) setFormTenantIds([...formTenantIds, t.id]);
                      else setFormTenantIds(formTenantIds.filter((id) => id !== t.id));
                    }}
                  />
                  {t.name}
                </label>
              ))}
            </div>
          </div>
        </div>
      </Drawer>
    </div>
  );
}
