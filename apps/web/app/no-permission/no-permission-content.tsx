"use client";

import { useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { fetchApi, handleApiError } from "@/lib/api-client";

interface AccessDeniedContext {
  reason: string;
  title: string;
  description: string;
  recoveryAction: string;
  returnTo: string;
  system: { id: string; code: string; name: string; status: string } | null;
  tenant: { id: string; code: string; name: string; status: string } | null;
}

export function NoPermissionContent() {
  const searchParams = useSearchParams();
  const reason = searchParams.get("reason") || "no_permission";
  const systemCode = searchParams.get("systemCode") || "";
  const tenantId = searchParams.get("tenantId") || "";
  const returnTo = searchParams.get("returnTo") || "/";

  const [context, setContext] = useState<AccessDeniedContext | null>(null);
  const [requested, setRequested] = useState(false);
  const [requesting, setRequesting] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    const params = new URLSearchParams();
    params.set("reason", reason);
    if (systemCode) params.set("systemCode", systemCode);
    if (tenantId) params.set("tenantId", tenantId);
    params.set("returnTo", returnTo);
    fetchApi<AccessDeniedContext>(`/api/access-denied/context?${params.toString()}`)
      .then(setContext)
      .catch((err: unknown) => {
        const { message } = handleApiError(err);
        setError(message);
      });
  }, [reason, systemCode, tenantId, returnTo]);

  async function recover() {
    if (reason === "session_expired") {
      window.location.href = `/login?redirectTo=${encodeURIComponent(returnTo)}`;
      return;
    }
    setRequesting(true);
    setError("");
    try {
      await fetchApi<unknown>("/api/permission-requests", {
        method: "POST",
        body: JSON.stringify({ reason, systemCode, tenantId, returnTo }),
      });
      setRequested(true);
    } catch (err: unknown) {
      const { message } = handleApiError(err);
      setError(message);
    } finally {
      setRequesting(false);
    }
  }

  if (!context) {
    return (
      <main className="flex min-h-screen items-center justify-center text-text-muted">
        {error || "加载中..."}
      </main>
    );
  }

  return (
    <main className="flex min-h-screen flex-col items-center justify-center bg-bg-primary p-6">
      <div className="w-full max-w-lg rounded-radius-lg border border-border-subtle bg-bg-secondary p-10 text-center">
        <h1 className="text-[34px] font-semibold leading-tight text-text-primary">{context.title}</h1>
        <p className="mt-3 text-[17px] leading-[1.47] text-text-secondary">{context.description}</p>

        <div className="mt-5 flex flex-col gap-2 text-left text-sm text-text-secondary">
          {context.system && (
            <div className="rounded-radius-sm bg-bg-tertiary p-4">
              <span className="text-text-muted">系统</span>
              <div className="mt-1 flex items-center gap-2">
                <span className="font-medium text-text-primary">{context.system.name}</span>
                <Badge variant="default">{context.system.status}</Badge>
              </div>
            </div>
          )}
          {context.tenant && (
            <div className="rounded-radius-sm bg-bg-tertiary p-4">
              <span className="text-text-muted">租户</span>
              <div className="mt-1 flex items-center gap-2">
                <span className="font-medium text-text-primary">{context.tenant.name}</span>
                <Badge variant="default">{context.tenant.status}</Badge>
              </div>
            </div>
          )}
        </div>

        {error && <p className="mt-4 text-sm text-color-error">{error}</p>}

        <div className="mt-6 space-y-3">
          {requested ? (
            <p className="text-sm text-status-success">权限申请已记录，请等待管理员处理。</p>
          ) : (
            <Button className="w-full" onClick={recover} disabled={requesting}>
              {requesting ? "处理中..." : context.recoveryAction}
            </Button>
          )}
          <Button variant="secondary" className="w-full" onClick={() => (window.location.href = returnTo)}>
            返回
          </Button>
          <Button variant="ghost" className="w-full" onClick={() => (window.location.href = "/")}>
            返回门户首页
          </Button>
        </div>
      </div>
    </main>
  );
}
