"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { TopBar } from "@/components/portal/top-bar";
import { SystemCard } from "@/components/portal/system-card";
import { Button } from "@/components/ui/button";
import { fetchApi, isAuthError, isPermissionError } from "@/lib/api-client";

interface HomeData {
  currentTenant: { id: string; code: string; name: string } | null;
  availableTenants: { id: string; code: string; name: string }[];
  user: { id: string; displayName: string; avatarUrl: string | null; canEnterAdmin: boolean };
  groups: { key: string; title: string; systems: any[] }[];
  noTenant?: boolean;
  allowPermissionRequest?: boolean;
}

export default function PortalHomePage() {
  const router = useRouter();
  const [data, setData] = useState<HomeData | null>(null);
  const [tenantId, setTenantId] = useState<string>("");
  const [loading, setLoading] = useState(true);

  async function load(tId?: string) {
    setLoading(true);
    try {
      const url = tId ? `/api/portal/home?tenantId=${tId}` : "/api/portal/home";
      const homeData = await fetchApi<HomeData>(url);
      setData(homeData);
      if (homeData.currentTenant) {
        setTenantId(homeData.currentTenant.id);
      }
    } catch (err: unknown) {
      if (isAuthError(err)) {
        router.push("/login");
        return;
      }
      if (isPermissionError(err)) {
        router.push("/no-permission");
        return;
      }
      // ignore other errors and keep loading state to avoid flash
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    load();
  }, []);

  if (loading || !data) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-bg-primary">
        <p className="text-text-muted">加载中...</p>
      </div>
    );
  }

  return (
    <div className="flex min-h-screen flex-col bg-bg-primary">
      <TopBar
        displayName={data.user.displayName}
        tenantName={data.currentTenant?.name}
        canEnterAdmin={data.user.canEnterAdmin}
      />

      <main className="flex-1 p-6">
        <div className="mx-auto max-w-6xl">
          <div className="mb-6 flex items-center justify-between">
            <h2 className="text-lg font-semibold text-text-primary">系统入口</h2>
            {data.availableTenants.length > 1 && (
              <select
                value={tenantId}
                onChange={(e) => {
                  setTenantId(e.target.value);
                  load(e.target.value);
                }}
                className="h-9 rounded-radius-sm border border-border-subtle bg-bg-secondary px-3 text-sm text-text-primary"
              >
                {data.availableTenants.map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.name}
                  </option>
                ))}
              </select>
            )}
          </div>

          {data.noTenant ? (
            <div className="rounded-radius-md border border-border-subtle bg-bg-secondary p-10 text-center">
              <p className="text-text-secondary">当前账号尚未加入租户</p>
            </div>
          ) : (
            <div className="space-y-8">
              {data.groups.map((group) =>
                group.systems.length === 0 ? null : (
                  <section key={group.key}>
                    <h3 className="mb-3 text-sm font-medium uppercase tracking-wide text-text-muted">
                      {group.title}
                    </h3>
                    <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
                      {group.systems.map((s) => (
                        <SystemCard key={s.systemCode} {...s} />
                      ))}
                    </div>
                  </section>
                )
              )}
              {data.groups.every((g) => g.systems.length === 0) && (
                <div className="rounded-radius-md border border-border-subtle bg-bg-secondary p-10 text-center">
                  <p className="text-text-secondary">暂无可访问系统</p>
                  {data.allowPermissionRequest && (
                    <Button variant="secondary" className="mt-4">
                      申请权限
                    </Button>
                  )}
                </div>
              )}
            </div>
          )}
        </div>
      </main>
    </div>
  );
}
