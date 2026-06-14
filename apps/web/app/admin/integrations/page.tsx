"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Drawer } from "@/components/admin/drawer";
import { SelectField } from "@/components/admin/select-field";
import { Textarea } from "@/components/admin/textarea";
import { CodeBlock } from "@/components/admin/code-block";
import { fetchApi, handleApiError, isAuthError, isPermissionError } from "@/lib/api-client";

interface Integration {
  id: string;
  code: string;
  name: string;
  description?: string;
  category?: string;
  status: string;
  entryUrl: string;
  callbackUrl?: string;
  authEnabled: boolean;
  supportsSubAdmin: boolean;
  integration: {
    issuer: string;
    authMode: string;
    tokenTtlSeconds: number;
    verifyEndpoint?: string;
    lastCheckAt?: string;
    lastCheckResult?: unknown;
  } | null;
}

interface IntegrationDetail extends Integration {
  supportedIdentityLevels: string[];
  supportedPermissions: string[];
  supportedScopes: unknown[];
  integration:
    | (Integration["integration"] & {
        publicKey?: string;
        envTemplate: Record<string, string>;
        envExample: string;
      })
    | null;
}

const authModes = [
  { value: "jwt", label: "JWT" },
  { value: "authorization_code", label: "Authorization Code" },
];

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

export default function IntegrationsPage() {
  const [items, setItems] = useState<Integration[]>([]);
  const [selected, setSelected] = useState<IntegrationDetail | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [error, setError] = useState("");

  async function loadItems() {
    try {
      const data = await fetchApi<Integration[]>("/api/admin/integrations");
      setItems(data);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      setError(message);
    }
  }

  useEffect(() => {
    loadItems();
  }, []);

  async function openDetail(id: string) {
    try {
      const data = await fetchApi<IntegrationDetail>(`/api/admin/integrations/${id}`);
      setSelected(data);
      setDrawerOpen(true);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    }
  }

  return (
    <div className="space-y-4">
      <h1 className="text-xl font-semibold text-text-primary">子系统接入</h1>

      {error && <p className="text-color-error">{error}</p>}

      <div className="overflow-auto rounded-radius-md border border-border-subtle">
        <table className="w-full text-left text-sm">
          <thead className="bg-bg-tertiary text-text-secondary">
            <tr>
              <th className="px-3 py-2 font-medium">系统</th>
              <th className="px-3 py-2 font-medium">编码</th>
              <th className="px-3 py-2 font-medium">状态</th>
              <th className="px-3 py-2 font-medium">认证</th>
              <th className="px-3 py-2 font-medium">最近检查</th>
              <th className="px-3 py-2 font-medium">操作</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border-subtle">
            {items.map((item) => (
              <tr key={item.id} className="hover:bg-hover-bg">
                <td className="px-3 py-2 text-text-primary">{item.name}</td>
                <td className="px-3 py-2 text-text-secondary">{item.code}</td>
                <td className="px-3 py-2">
                  <Badge variant={item.status === "active" ? "success" : item.status === "maintenance" ? "warning" : "default"}>
                    {item.status}
                  </Badge>
                </td>
                <td className="px-3 py-2 text-text-secondary">{item.integration?.authMode || "-"}</td>
                <td className="px-3 py-2 text-text-secondary">
                  {item.integration?.lastCheckAt ? new Date(item.integration.lastCheckAt).toLocaleString() : "未检查"}
                </td>
                <td className="px-3 py-2">
                  <Button variant="ghost" onClick={() => openDetail(item.id)}>
                    配置
                  </Button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {selected && (
        <IntegrationDrawer
          key={selected.id}
          open={drawerOpen}
          onClose={() => setDrawerOpen(false)}
          data={selected}
          onSaved={loadItems}
        />
      )}
    </div>
  );
}

function IntegrationDrawer({
  open,
  onClose,
  data,
  onSaved,
}: {
  open: boolean;
  onClose: () => void;
  data: IntegrationDetail;
  onSaved: () => void;
}) {
  const [issuer, setIssuer] = useState(data.integration?.issuer || "");
  const [authMode, setAuthMode] = useState(data.integration?.authMode || "authorization_code");
  const [tokenTtl, setTokenTtl] = useState(data.integration?.tokenTtlSeconds || 300);
  const [publicKey, setPublicKey] = useState(data.integration?.publicKey || "");
  const [verifyEndpoint, setVerifyEndpoint] = useState(data.integration?.verifyEndpoint || "");
  const [callbackUrl, setCallbackUrl] = useState(data.callbackUrl || "");
  const [envTemplate, setEnvTemplate] = useState(
    data.integration
      ? Object.entries(data.integration.envTemplate || {})
          .map(([k, v]) => `${k}=${v}`)
          .join("\n")
      : ""
  );
  const [checkResult, setCheckResult] = useState<unknown>(data.integration?.lastCheckResult || null);
  const [submitting, setSubmitting] = useState(false);

  async function save() {
    if (!data) return;
    setSubmitting(true);
    const env: Record<string, string> = {};
    envTemplate.split("\n").forEach((line) => {
      const idx = line.indexOf("=");
      if (idx > 0) env[line.slice(0, idx)] = line.slice(idx + 1);
    });

    try {
      await fetchApi<unknown>(`/api/admin/integrations/${data.id}`, {
        method: "PUT",
        body: JSON.stringify({
          issuer,
          authMode,
          tokenTtlSeconds: Number(tokenTtl),
          publicKey,
          verifyEndpoint,
          callbackUrl,
          envTemplate: env,
        }),
      });
      onSaved();
      onClose();
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    } finally {
      setSubmitting(false);
    }
  }

  async function check() {
    if (!data) return;
    try {
      const result = await fetchApi<unknown>(`/api/admin/integrations/${data.id}/check`, {
        method: "POST",
        body: JSON.stringify({ systemCode: data.code, callbackUrl, authMode }),
      });
      setCheckResult(result);
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    }
  }

  return (
    <Drawer
      open={open}
      onClose={onClose}
      title={`${data.name} 接入配置`}
      footer={
        <div className="flex justify-end gap-2">
          <Button variant="secondary" onClick={onClose}>
            关闭
          </Button>
          <Button variant="secondary" onClick={check}>
            接入检查
          </Button>
          <Button onClick={save} disabled={submitting}>
            保存
          </Button>
        </div>
      }
    >
      <div className="space-y-4">
        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="text-sm font-medium text-text-secondary">Issuer</label>
            <Input value={issuer} onChange={(e) => setIssuer(e.target.value)} />
          </div>
          <div>
            <SelectField label="校验方式" value={authMode} onChange={(e) => setAuthMode(e.target.value)} options={authModes} />
          </div>
        </div>
        <div>
          <label className="text-sm font-medium text-text-secondary">Token 有效期（秒）</label>
          <Input type="number" value={tokenTtl} onChange={(e) => setTokenTtl(Number(e.target.value))} />
        </div>
        <div>
          <label className="text-sm font-medium text-text-secondary">回调地址</label>
          <Input value={callbackUrl} onChange={(e) => setCallbackUrl(e.target.value)} />
        </div>
        <div>
          <label className="text-sm font-medium text-text-secondary">公钥 / JWKS</label>
          <Textarea value={publicKey} onChange={(e) => setPublicKey(e.target.value)} />
        </div>
        <div>
          <label className="text-sm font-medium text-text-secondary">校验端点</label>
          <Input value={verifyEndpoint} onChange={(e) => setVerifyEndpoint(e.target.value)} />
        </div>
        <div>
          <label className="text-sm font-medium text-text-secondary">环境变量模板（KEY=VALUE 每行）</label>
          <Textarea value={envTemplate} onChange={(e) => setEnvTemplate(e.target.value)} />
        </div>
        <div>
          <label className="text-sm font-medium text-text-secondary">环境变量示例</label>
          <CodeBlock code={data.integration?.envExample || "# 未配置"} language="env" />
        </div>
        {checkResult ? (
          <div className="rounded-radius-sm border border-border-subtle bg-bg-tertiary p-3 text-sm">
            <p className="font-medium text-text-primary">
              检查结果：
              {(checkResult as { passed?: boolean }).passed ? (
                <span className="text-status-success">通过</span>
              ) : (
                <span className="text-status-error">未通过</span>
              )}
            </p>
            <pre className="mt-2 text-xs text-text-secondary">
              {JSON.stringify((checkResult as { checks?: unknown }).checks, null, 2)}
            </pre>
          </div>
        ) : null}
      </div>
    </Drawer>
  );
}
