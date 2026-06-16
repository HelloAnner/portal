"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { TopBar } from "@/components/portal/top-bar";
import { AdminSidebar } from "@/components/admin/sidebar";
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
    <div className="flex min-h-screen bg-bg-primary">
      <AdminSidebar
        showAdmin={me.canEnterAdmin}
        displayName={me.displayName}
        roleLabel={me.canEnterAdmin ? "Admin" : "User"}
      />
      <div className="flex min-w-0 flex-1 flex-col">
      <TopBar
        title="我的权限"
        displayName={me.displayName}
        tenantName={me.tenants?.[0]?.name}
        canEnterAdmin={me.canEnterAdmin}
      />

      <main className="flex-1 px-8 pb-8">
        <div className="mx-auto max-w-6xl">
          <div className="mb-6">
            <h1 className="text-[34px] font-semibold leading-tight text-text-primary">我的权限</h1>
            <p className="mt-2 text-[17px] leading-[1.47] text-text-secondary">
              查看门户角色、租户范围、系统身份和权限来源。
            </p>
          </div>

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
    </div>
  );
}
