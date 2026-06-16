"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { TopBar } from "@/components/portal/top-bar";
import { AdminSidebar } from "@/components/admin/sidebar";
import { ProfileCard } from "@/components/portal/profile-card";
import { ProfileEditor } from "@/components/portal/profile-editor";
import { fetchApi, isAuthError } from "@/lib/api-client";

interface ProfileData {
  username: string;
  displayName: string;
  email: string | null;
  phone: string | null;
  organizationPath: string | null;
  status: string;
  avatarUrl: string | null;
  defaultTenantId: string | null;
  preferences: Record<string, unknown>;
}

interface MeData {
  id: string;
  displayName: string;
  avatarUrl: string | null;
  canEnterAdmin: boolean;
  tenants: { id: string; code: string; name: string }[];
}

export default function ProfilePage() {
  const router = useRouter();
  const [profile, setProfile] = useState<ProfileData | null>(null);
  const [me, setMe] = useState<MeData | null>(null);
  const [loading, setLoading] = useState(true);

  async function load() {
    setLoading(true);
    try {
      const [profileData, meData] = await Promise.all([
        fetchApi<ProfileData>("/api/profile"),
        fetchApi<MeData>("/api/auth/me"),
      ]);
      setProfile(profileData);
      setMe(meData);
    } catch (err: unknown) {
      if (isAuthError(err)) {
        router.push("/login?redirectTo=/profile");
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

  if (loading || !profile || !me) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-bg-primary">
        <p className="text-text-muted">加载中...</p>
      </div>
    );
  }

  return (
    <div className="flex min-h-screen bg-bg-primary">
      <AdminSidebar
        showAdmin={me.canEnterAdmin}
        displayName={me.displayName}
        roleLabel={me.canEnterAdmin ? "Admin" : "User"}
      />
      <div className="flex min-w-0 flex-1 flex-col">
      <TopBar
        title="我的资料"
        displayName={me.displayName}
        tenantName={me.tenants?.[0]?.name}
        canEnterAdmin={me.canEnterAdmin}
      />

      <main className="flex-1 px-8 pb-8">
        <div className="mx-auto max-w-6xl">
          <div className="mb-6">
            <h1 className="text-[34px] font-semibold leading-tight text-text-primary">我的资料</h1>
            <p className="mt-2 text-[17px] leading-[1.47] text-text-secondary">
              维护展示资料、头像和默认进入偏好。
            </p>
          </div>

          <div className="grid grid-cols-1 gap-6 md:grid-cols-3">
            <div className="md:col-span-1">
              <ProfileCard
                username={profile.username}
                displayName={profile.displayName}
                email={profile.email}
                phone={profile.phone}
                organizationPath={profile.organizationPath}
                status={profile.status}
                avatarUrl={profile.avatarUrl}
              />
            </div>
            <div className="md:col-span-2">
              <ProfileEditor
                displayName={profile.displayName}
                avatarUrl={profile.avatarUrl}
                defaultTenantId={profile.defaultTenantId}
                preferences={profile.preferences || {}}
                tenants={me.tenants}
                onSaved={load}
              />
            </div>
          </div>
        </div>
      </main>
      </div>
    </div>
  );
}
