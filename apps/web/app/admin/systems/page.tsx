"use client";

import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Drawer } from "@/components/admin/drawer";
import { Modal } from "@/components/admin/modal";
import { ChevronLeft, ChevronRight, Plus, Search, X } from "lucide-react";
import { fetchApi, handleApiError, isAuthError, isPermissionError } from "@/lib/api-client";

interface SystemRow {
  id: string;
  code: string;
  name: string;
  status: string;
  category: string | null;
  entryUrl: string;
  tenantCount: number;
  assignmentCount: number;
  createdAt: string;
}

interface SystemListResponse {
  items: SystemRow[];
  pagination: { total: number };
}

interface SystemDetail {
  system: {
    id: string;
    code: string;
    name: string;
    description: string | null;
    category: string | null;
    iconUrl: string | null;
    entryUrl: string;
    callbackUrl: string | null;
    status: string;
    portalManaged: boolean;
    authEnabled: boolean;
    supportsSubAdmin: boolean;
    supportedIdentityLevels: string[];
    supportedPermissions: string[];
    supportedScopes: Array<{ scopeType: string; scopeCode: string; name?: string }>;
    integrationConfig: {
      issuer: string;
      authMode: string;
      tokenTtlSeconds: number;
      publicKey: string;
      verifyEndpoint: string;
      envTemplate: Record<string, string>;
    };
  };
  tenantSystems: Array<{
    id: string;
    enabled: boolean;
    tenant: { id: string; code: string; name: string };
  }>;
  auditEvents: any[];
}

const statusOptions = [
  { value: "", label: "全部状态" },
  { value: "active", label: "正常" },
  { value: "disabled", label: "已停用" },
  { value: "onboarding", label: "接入中" },
  { value: "maintenance", label: "维护中" },
];

const identityLevelOptions = ["user", "tenant-admin", "super-admin"];

function emptySystemForm(): any {
  return {
    code: "",
    name: "",
    description: "",
    category: "",
    iconUrl: "",
    entryUrl: "",
    callbackUrl: "",
    status: "active",
    portalManaged: true,
    authEnabled: true,
    supportsSubAdmin: false,
    supportedIdentityLevels: ["user"],
    supportedPermissions: [],
    supportedScopes: [],
    integrationConfig: {
      issuer: "http://localhost:8080",
      authMode: "jwt",
      tokenTtlSeconds: 300,
      publicKey: "",
      verifyEndpoint: "",
      envTemplate: {},
    },
  };
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

export default function SystemsPage() {
  const [items, setItems] = useState<SystemRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [page, setPage] = useState(1);
  const [pageSize] = useState(15);
  const [total, setTotal] = useState(0);

  const [keyword, setKeyword] = useState("");
  const [category, setCategory] = useState("");
  const [status, setStatus] = useState("");

  const [detail, setDetail] = useState<SystemDetail | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<"info" | "tenants" | "integration" | "audit">("info");

  const [createOpen, setCreateOpen] = useState(false);
  const [createStep, setCreateStep] = useState(1);
  const [form, setForm] = useState<any>(emptySystemForm());
  const [saving, setSaving] = useState(false);

  const totalPages = useMemo(() => Math.max(1, Math.ceil(total / pageSize)), [total, pageSize]);

  async function fetchSystems(targetPage = page) {
    setLoading(true);
    const params = new URLSearchParams();
    params.set("page", String(targetPage));
    params.set("pageSize", String(pageSize));
    if (keyword) params.set("keyword", keyword);
    if (category) params.set("category", category);
    if (status) params.set("status", status);
    try {
      const data = await fetchApi<SystemListResponse>(`/api/admin/systems?${params.toString()}`);
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
    fetchSystems(page);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [page, keyword, category, status]);

  async function openDetail(system: { id: string }) {
    try {
      const data = await fetchApi<SystemDetail>(`/api/admin/systems/${system.id}`);
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
      await fetchApi<unknown>(`/api/admin/systems/${detail.system.id}`, {
        method: "PATCH",
        body: JSON.stringify({
          name: detail.system.name,
          description: detail.system.description,
          category: detail.system.category,
          iconUrl: detail.system.iconUrl,
          entryUrl: detail.system.entryUrl,
          callbackUrl: detail.system.callbackUrl,
          portalManaged: detail.system.portalManaged,
          authEnabled: detail.system.authEnabled,
          supportsSubAdmin: detail.system.supportsSubAdmin,
          supportedIdentityLevels: detail.system.supportedIdentityLevels,
          supportedPermissions: detail.system.supportedPermissions,
          supportedScopes: detail.system.supportedScopes,
        }),
      });
      openDetail(detail.system);
      fetchSystems(page);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    }
  }

  async function saveIntegration() {
    if (!detail) return;
    try {
      await fetchApi<unknown>(`/api/admin/systems/${detail.system.id}`, {
        method: "PATCH",
        body: JSON.stringify({
          integrationConfig: detail.system.integrationConfig,
        }),
      });
      openDetail(detail.system);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    }
  }

  async function toggleStatus() {
    if (!detail) return;
    try {
      await fetchApi<unknown>(`/api/admin/systems/${detail.system.id}/status`, {
        method: "POST",
      });
      openDetail(detail.system);
      fetchSystems(page);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    }
  }

  async function createSystem() {
    setSaving(true);
    try {
      await fetchApi<unknown>("/api/admin/systems", {
        method: "POST",
        body: JSON.stringify(form),
      });
      setCreateOpen(false);
      setForm(emptySystemForm());
      setCreateStep(1);
      fetchSystems(1);
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
      status === "active"
        ? "success"
        : status === "disabled"
        ? "error"
        : status === "maintenance"
        ? "warning"
        : "info";
    const label = statusOptions.find((s) => s.value === status)?.label || status;
    return <Badge variant={variant as any}>{label}</Badge>;
  }

  function updateForm(patch: any) {
    setForm({ ...form, ...patch });
  }

  function updateIntegration(patch: any) {
    setForm({
      ...form,
      integrationConfig: { ...form.integrationConfig, ...patch },
    });
  }

  function updateDetailIntegration(patch: any) {
    if (!detail) return;
    setDetail({
      ...detail,
      system: {
        ...detail.system,
        integrationConfig: { ...detail.system.integrationConfig, ...patch },
      },
    });
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold text-text-primary">系统目录</h1>
        <Button onClick={() => setCreateOpen(true)}>
          <Plus className="h-4 w-4" />
          新增系统
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
          <label className="text-xs text-text-muted">分类</label>
          <Input
            placeholder="分类"
            value={category}
            onChange={(e) => setCategory(e.target.value)}
            className="w-40"
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
        <Button variant="secondary" onClick={() => fetchSystems(1)}>
          <Search className="h-4 w-4" />
          搜索
        </Button>
        <Button variant="ghost" onClick={() => { setKeyword(""); setCategory(""); setStatus(""); setPage(1); }}>
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
              <th className="px-4 py-3">分类</th>
              <th className="px-4 py-3">状态</th>
              <th className="px-4 py-3">入口地址</th>
              <th className="px-4 py-3">租户数</th>
              <th className="px-4 py-3 text-right">操作</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border-faint">
            {items.map((s) => (
              <tr key={s.id} className="hover:bg-hover-bg">
                <td className="px-4 py-3 font-medium text-text-primary">{s.code}</td>
                <td className="px-4 py-3 text-text-secondary">{s.name}</td>
                <td className="px-4 py-3 text-text-secondary">{s.category || "-"}</td>
                <td className="px-4 py-3">{statusBadge(s.status)}</td>
                <td className="px-4 py-3 text-text-muted max-w-xs truncate">{s.entryUrl}</td>
                <td className="px-4 py-3 text-text-secondary">{s.tenantCount}</td>
                <td className="px-4 py-3 text-right">
                  <Button variant="ghost" onClick={() => openDetail(s)}>
                    详情
                  </Button>
                </td>
              </tr>
            ))}
            {items.length === 0 && !loading && (
              <tr>
                <td colSpan={7} className="px-4 py-8 text-center text-text-muted">
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
        title={detail?.system?.name || "系统详情"}
        width="640px"
      >
        {detail && (
          <div className="space-y-5">
            <div className="flex gap-2 border-b border-border-subtle pb-2">
              {([
                { key: "info", label: "基础信息" },
                { key: "tenants", label: "接入租户" },
                { key: "integration", label: "接入配置" },
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
                    <div className="text-sm text-text-primary">{detail.system.code}</div>
                  </div>
                  <div>
                    <label className="text-xs text-text-muted">状态</label>
                    <div className="flex items-center gap-2">
                      {statusBadge(detail.system.status)}
                      <Button variant="secondary" onClick={toggleStatus}>
                        切换状态
                      </Button>
                    </div>
                  </div>
                </div>
                <div>
                  <label className="text-xs text-text-muted">名称</label>
                  <Input
                    value={detail.system.name}
                    onChange={(e) => setDetail({ ...detail, system: { ...detail.system, name: e.target.value } })}
                  />
                </div>
                <div>
                  <label className="text-xs text-text-muted">描述</label>
                  <Input
                    value={detail.system.description || ""}
                    onChange={(e) => setDetail({ ...detail, system: { ...detail.system, description: e.target.value } })}
                  />
                </div>
                <div>
                  <label className="text-xs text-text-muted">分类</label>
                  <Input
                    value={detail.system.category || ""}
                    onChange={(e) => setDetail({ ...detail, system: { ...detail.system, category: e.target.value } })}
                  />
                </div>
                <div>
                  <label className="text-xs text-text-muted">入口地址</label>
                  <Input
                    value={detail.system.entryUrl}
                    onChange={(e) => setDetail({ ...detail, system: { ...detail.system, entryUrl: e.target.value } })}
                  />
                </div>
                <div>
                  <label className="text-xs text-text-muted">回调地址</label>
                  <Input
                    value={detail.system.callbackUrl || ""}
                    onChange={(e) => setDetail({ ...detail, system: { ...detail.system, callbackUrl: e.target.value } })}
                  />
                </div>
                <div>
                  <label className="text-xs text-text-muted">图标地址</label>
                  <Input
                    value={detail.system.iconUrl || ""}
                    onChange={(e) => setDetail({ ...detail, system: { ...detail.system, iconUrl: e.target.value } })}
                  />
                </div>
                <div className="flex gap-4 text-sm text-text-primary">
                  <label className="flex items-center gap-1">
                    <input
                      type="checkbox"
                      checked={detail.system.portalManaged}
                      onChange={(e) => setDetail({ ...detail, system: { ...detail.system, portalManaged: e.target.checked } })}
                    />
                    门户托管
                  </label>
                  <label className="flex items-center gap-1">
                    <input
                      type="checkbox"
                      checked={detail.system.authEnabled}
                      onChange={(e) => setDetail({ ...detail, system: { ...detail.system, authEnabled: e.target.checked } })}
                    />
                    启用认证
                  </label>
                  <label className="flex items-center gap-1">
                    <input
                      type="checkbox"
                      checked={detail.system.supportsSubAdmin}
                      onChange={(e) => setDetail({ ...detail, system: { ...detail.system, supportsSubAdmin: e.target.checked } })}
                    />
                    支持子管理员
                  </label>
                </div>
                <div>
                  <label className="text-xs text-text-muted">支持身份级别</label>
                  <div className="mt-1 flex flex-wrap gap-2">
                    {identityLevelOptions.map((lvl) => (
                      <label key={lvl} className="flex items-center gap-1 text-sm text-text-primary">
                        <input
                          type="checkbox"
                          checked={detail.system.supportedIdentityLevels.includes(lvl)}
                          onChange={(e) => {
                            const next = e.target.checked
                              ? [...detail.system.supportedIdentityLevels, lvl]
                              : detail.system.supportedIdentityLevels.filter((x: string) => x !== lvl);
                            setDetail({ ...detail, system: { ...detail.system, supportedIdentityLevels: next } });
                          }}
                        />
                        {lvl}
                      </label>
                    ))}
                  </div>
                </div>
                <Button onClick={saveInfo}>保存基础信息</Button>
              </div>
            )}

            {activeTab === "tenants" && (
              <div className="space-y-2">
                {detail.tenantSystems.map((ts) => (
                  <div key={ts.id} className="flex items-center justify-between rounded-radius-sm border border-border-subtle p-3">
                    <div>
                      <div className="text-sm font-medium text-text-primary">{ts.tenant.name}</div>
                      <div className="text-xs text-text-muted">{ts.tenant.code}</div>
                    </div>
                    <Badge variant={ts.enabled ? "success" : "default"}>{ts.enabled ? "已启用" : "已禁用"}</Badge>
                  </div>
                ))}
              </div>
            )}

            {activeTab === "integration" && (
              <div className="space-y-4">
                <div>
                  <label className="text-xs text-text-muted">Issuer</label>
                  <Input
                    value={detail.system.integrationConfig?.issuer || ""}
                    onChange={(e) => updateDetailIntegration({ issuer: e.target.value })}
                  />
                </div>
                <div>
                  <label className="text-xs text-text-muted">认证模式</label>
                  <select
                    value={detail.system.integrationConfig?.authMode || "jwt"}
                    onChange={(e) => updateDetailIntegration({ authMode: e.target.value })}
                    className="h-10 w-full rounded-radius-sm border border-border-subtle bg-bg-secondary px-3 text-sm text-text-primary"
                  >
                    <option value="jwt">JWT</option>
                    <option value="authorization_code">授权码</option>
                  </select>
                </div>
                <div>
                  <label className="text-xs text-text-muted">Token TTL（秒）</label>
                  <Input
                    type="number"
                    value={detail.system.integrationConfig?.tokenTtlSeconds || 300}
                    onChange={(e) => updateDetailIntegration({ tokenTtlSeconds: Number(e.target.value) })}
                  />
                </div>
                <div>
                  <label className="text-xs text-text-muted">公钥路径/内容</label>
                  <Input
                    value={detail.system.integrationConfig?.publicKey || ""}
                    onChange={(e) => updateDetailIntegration({ publicKey: e.target.value })}
                  />
                </div>
                <div>
                  <label className="text-xs text-text-muted">校验端点</label>
                  <Input
                    value={detail.system.integrationConfig?.verifyEndpoint || ""}
                    onChange={(e) => updateDetailIntegration({ verifyEndpoint: e.target.value })}
                  />
                </div>
                <Button onClick={saveIntegration}>保存接入配置</Button>
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

      <Modal
        open={createOpen}
        onClose={() => setCreateOpen(false)}
        title="新增系统"
        maxWidth="640px"
        footer={
          <>
            <Button variant="secondary" onClick={() => setCreateOpen(false)}>
              取消
            </Button>
            {createStep > 1 && (
              <Button variant="secondary" onClick={() => setCreateStep(createStep - 1)}>
                上一步
              </Button>
            )}
            {createStep < 4 && (
              <Button onClick={() => setCreateStep(createStep + 1)}>下一步</Button>
            )}
            {createStep === 4 && (
              <Button onClick={createSystem} disabled={saving}>
                {saving ? "创建中..." : "创建"}
              </Button>
            )}
          </>
        }
      >
        <div className="mb-4 flex items-center gap-2 text-sm text-text-secondary">
          {[1, 2, 3, 4].map((s) => (
            <span
              key={s}
              className={`flex h-7 w-7 items-center justify-center rounded-full ${
                createStep === s
                  ? "bg-text-primary text-bg-secondary"
                  : createStep > s
                  ? "bg-bg-tertiary text-text-primary"
                  : "bg-bg-tertiary text-text-muted"
              }`}
            >
              {s}
            </span>
          ))}
        </div>

        {createStep === 1 && (
          <div className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm text-text-secondary">系统编码</label>
              <Input value={form.code} onChange={(e) => updateForm({ code: e.target.value })} />
            </div>
            <div className="space-y-2">
              <label className="text-sm text-text-secondary">系统名称</label>
              <Input value={form.name} onChange={(e) => updateForm({ name: e.target.value })} />
            </div>
            <div className="space-y-2">
              <label className="text-sm text-text-secondary">描述</label>
              <Input value={form.description} onChange={(e) => updateForm({ description: e.target.value })} />
            </div>
            <div className="space-y-2">
              <label className="text-sm text-text-secondary">状态</label>
              <select
                value={form.status}
                onChange={(e) => updateForm({ status: e.target.value })}
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
        )}

        {createStep === 2 && (
          <div className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm text-text-secondary">分类</label>
              <Input value={form.category} onChange={(e) => updateForm({ category: e.target.value })} />
            </div>
            <div className="space-y-2">
              <label className="text-sm text-text-secondary">入口地址</label>
              <Input value={form.entryUrl} onChange={(e) => updateForm({ entryUrl: e.target.value })} />
            </div>
            <div className="space-y-2">
              <label className="text-sm text-text-secondary">回调地址</label>
              <Input value={form.callbackUrl} onChange={(e) => updateForm({ callbackUrl: e.target.value })} />
            </div>
            <div className="space-y-2">
              <label className="text-sm text-text-secondary">图标地址</label>
              <Input value={form.iconUrl} onChange={(e) => updateForm({ iconUrl: e.target.value })} />
            </div>
            <div className="flex gap-4 text-sm text-text-primary">
              <label className="flex items-center gap-1">
                <input type="checkbox" checked={form.portalManaged} onChange={(e) => updateForm({ portalManaged: e.target.checked })} />
                门户托管
              </label>
              <label className="flex items-center gap-1">
                <input type="checkbox" checked={form.authEnabled} onChange={(e) => updateForm({ authEnabled: e.target.checked })} />
                启用认证
              </label>
              <label className="flex items-center gap-1">
                <input type="checkbox" checked={form.supportsSubAdmin} onChange={(e) => updateForm({ supportsSubAdmin: e.target.checked })} />
                支持子管理员
              </label>
            </div>
            <div>
              <label className="text-sm text-text-secondary">支持身份级别</label>
              <div className="mt-1 flex flex-wrap gap-2">
                {identityLevelOptions.map((lvl) => (
                  <label key={lvl} className="flex items-center gap-1 text-sm text-text-primary">
                    <input
                      type="checkbox"
                      checked={form.supportedIdentityLevels.includes(lvl)}
                      onChange={(e) => {
                        const next = e.target.checked
                          ? [...form.supportedIdentityLevels, lvl]
                          : form.supportedIdentityLevels.filter((x: string) => x !== lvl);
                        updateForm({ supportedIdentityLevels: next });
                      }}
                    />
                    {lvl}
                  </label>
                ))}
              </div>
            </div>
          </div>
        )}

        {createStep === 3 && (
          <div className="space-y-4">
            <div>
              <label className="text-sm text-text-secondary">支持权限</label>
              <div className="mt-2 space-y-2">
                {form.supportedPermissions.map((p: string, idx: number) => (
                  <div key={idx} className="flex items-center gap-2">
                    <Input value={p} onChange={(e) => {
                      const next = [...form.supportedPermissions];
                      next[idx] = e.target.value;
                      updateForm({ supportedPermissions: next });
                    }} />
                    <Button variant="ghost" onClick={() => {
                      const next = form.supportedPermissions.filter((_: any, i: number) => i !== idx);
                      updateForm({ supportedPermissions: next });
                    }}>
                      <X className="h-4 w-4" />
                    </Button>
                  </div>
                ))}
                <Button variant="secondary" onClick={() => updateForm({ supportedPermissions: [...form.supportedPermissions, ""] })}>
                  <Plus className="h-4 w-4" />
                  添加权限
                </Button>
              </div>
            </div>
            <div>
              <label className="text-sm text-text-secondary">支持范围</label>
              <div className="mt-2 space-y-2">
                {form.supportedScopes.map((scope: any, idx: number) => (
                  <div key={idx} className="grid grid-cols-3 gap-2">
                    <Input placeholder="类型" value={scope.scopeType || ""} onChange={(e) => {
                      const next = [...form.supportedScopes];
                      next[idx] = { ...next[idx], scopeType: e.target.value };
                      updateForm({ supportedScopes: next });
                    }} />
                    <Input placeholder="编码" value={scope.scopeCode || ""} onChange={(e) => {
                      const next = [...form.supportedScopes];
                      next[idx] = { ...next[idx], scopeCode: e.target.value };
                      updateForm({ supportedScopes: next });
                    }} />
                    <Input placeholder="名称" value={scope.name || ""} onChange={(e) => {
                      const next = [...form.supportedScopes];
                      next[idx] = { ...next[idx], name: e.target.value };
                      updateForm({ supportedScopes: next });
                    }} />
                  </div>
                ))}
                <Button variant="secondary" onClick={() => updateForm({ supportedScopes: [...form.supportedScopes, { scopeType: "module", scopeCode: "", name: "" }] })}>
                  <Plus className="h-4 w-4" />
                  添加范围
                </Button>
              </div>
            </div>
          </div>
        )}

        {createStep === 4 && (
          <div className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm text-text-secondary">Issuer</label>
              <Input value={form.integrationConfig.issuer} onChange={(e) => updateIntegration({ issuer: e.target.value })} />
            </div>
            <div className="space-y-2">
              <label className="text-sm text-text-secondary">认证模式</label>
              <select
                value={form.integrationConfig.authMode}
                onChange={(e) => updateIntegration({ authMode: e.target.value })}
                className="h-10 w-full rounded-radius-sm border border-border-subtle bg-bg-secondary px-3 text-sm text-text-primary"
              >
                <option value="jwt">JWT</option>
                <option value="authorization_code">授权码</option>
              </select>
            </div>
            <div className="space-y-2">
              <label className="text-sm text-text-secondary">Token TTL（秒）</label>
              <Input type="number" value={form.integrationConfig.tokenTtlSeconds} onChange={(e) => updateIntegration({ tokenTtlSeconds: Number(e.target.value) })} />
            </div>
            <div className="space-y-2">
              <label className="text-sm text-text-secondary">公钥路径/内容</label>
              <Input value={form.integrationConfig.publicKey} onChange={(e) => updateIntegration({ publicKey: e.target.value })} />
            </div>
            <div className="space-y-2">
              <label className="text-sm text-text-secondary">校验端点</label>
              <Input value={form.integrationConfig.verifyEndpoint} onChange={(e) => updateIntegration({ verifyEndpoint: e.target.value })} />
            </div>
            <div className="space-y-2">
              <label className="text-sm text-text-secondary">环境变量模板（JSON）</label>
              <textarea
                value={JSON.stringify(form.integrationConfig.envTemplate, null, 2)}
                onChange={(e) => {
                  try {
                    updateIntegration({ envTemplate: JSON.parse(e.target.value) });
                  } catch {
                    updateIntegration({ envTemplate: e.target.value });
                  }
                }}
                className="h-32 w-full rounded-radius-sm border border-border-subtle bg-bg-secondary p-3 font-mono text-xs text-text-primary"
              />
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
}
