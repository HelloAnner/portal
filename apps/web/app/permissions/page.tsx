"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { TopBar } from "@/components/portal/top-bar";
import { PermissionList } from "@/components/portal/permission-list";
import { SideDrawer } from "@/components/portal/side-drawer";
import { SystemPermissionDetail } from "@/components/portal/system-permission-detail";
import { fetchApi, isAuthError, isPermissionError } from "@/lib/api-client";

interface TenantItem {
  id: string;
  code: string;
  name: string;
}

interface SystemPermissionItem {
  systemCode: string;
  name: string;
  description: string | null;
  iconUrl: string | null;
  status: string;
  category: string | null;
  tenantId: string;
  tenantName: string;
  identity: string;
  visible: boolean;
  accessible: boolean;
  sourceSummary: string[];
  scopeSummary: string[];
}

interface PermissionsData {
  portalRoles: string[];
  tenants: TenantItem[];
  systems: SystemPermissionItem[];
}

interface MeData {
  id: string;
  displayName: string;
  avatarUrl: string | null;
  canEnterAdmin: boolean;
  tenants: TenantItem[];
}

export default function PermissionsPage() {
  const router = useRouter();
  const [data, setData] = useState<PermissionsData | null>(null);
  const [me, setMe] = useState<MeData | null>(null);
  const [loading, setLoading] = useState(true);
  const [selectedSystemCode, setSelectedSystemCode] = useState<string | null>(null);

  async function load() {
    setLoading(true);
    try {
      const [permissionsData, meData] = await Promise.all([
        fetchApi<PermissionsData>("/api/me/permissions"),
        fetchApi<MeData>("/api/auth/me"),
      ]);
      setData(permissionsData);
      setMe(meData);
    } catch (err: unknown) {
      if (isAuthError(err)) {
        router.push("/login?redirectTo=/permissions");
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

  if (loading || !data || !me) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-bg-primary">
        <p className="text-text-muted">加载中...</p>
      </div>
    );
  }

  const selectedSystem = selectedSystemCode
    ? data.systems.find((s) => s.systemCode === selectedSystemCode)
    : null;

  return (
    <div className="flex min-h-screen flex-col bg-bg-primary">
      <TopBar
        displayName={me.displayName}
        tenantName={me.tenants?.[0]?.name}
        canEnterAdmin={me.canEnterAdmin}
      />

      <main className="flex-1 p-6">
        <div className="mx-auto max-w-5xl">
          <h1 className="mb-6 text-xl font-semibold text-text-primary">我的权限</h1>

          <PermissionList
            portalRoles={data.portalRoles}
            tenants={data.tenants}
            systems={data.systems}
            onSelectSystem={setSelectedSystemCode}
          />
        </div>
      </main>

      <SideDrawer
        open={!!selectedSystemCode}
        onClose={() => setSelectedSystemCode(null)}
        title={selectedSystem?.name || "系统权限详情"}
      >
        {selectedSystemCode && <SystemPermissionDetail systemCode={selectedSystemCode} />}
      </SideDrawer>
    </div>
  );
}
