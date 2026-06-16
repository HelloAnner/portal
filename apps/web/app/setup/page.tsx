"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { fetchApi, handleApiError } from "@/lib/api-client";

interface SetupStatus {
  initialized: boolean;
  needsSetup: boolean;
}

export default function SetupPage() {
  const router = useRouter();
  const [checking, setChecking] = useState(true);
  const [tenantName, setTenantName] = useState("默认租户");
  const [username, setUsername] = useState("admin");
  const [displayName, setDisplayName] = useState("系统管理员");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let mounted = true;
    fetchApi<SetupStatus>("/api/setup/status")
      .then((status) => {
        if (!mounted) return;
        if (!status.needsSetup) {
          router.replace("/login");
          return;
        }
        setChecking(false);
      })
      .catch(() => setChecking(false));
    return () => {
      mounted = false;
    };
  }, [router]);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError("");
    if (password.length < 8) {
      setError("密码至少需要 8 位");
      return;
    }
    if (password !== confirmPassword) {
      setError("两次输入的密码不一致");
      return;
    }
    setLoading(true);
    try {
      await fetchApi("/api/setup/bootstrap-super-admin", {
        method: "POST",
        body: JSON.stringify({
          tenantName,
          username,
          displayName,
          email: email || null,
          password,
        }),
      });
      router.replace("/");
      router.refresh();
    } catch (err) {
      const apiError = handleApiError(err);
      setError(apiError.code === "CONFLICT" ? "门户已经完成初始化" : "初始化失败，请检查输入后重试");
      setLoading(false);
    }
  }

  if (checking) {
    return (
      <main className="flex min-h-screen items-center justify-center bg-bg-primary">
        <p className="text-sm text-text-muted">检查初始化状态...</p>
      </main>
    );
  }

  return (
    <main className="min-h-screen bg-bg-primary px-6 py-10">
      <div className="mx-auto flex max-w-5xl gap-8">
        <section className="w-[360px] shrink-0 pt-10">
          <p className="text-sm font-medium text-text-muted">首次启动</p>
          <h1 className="mt-3 text-3xl font-semibold text-text-primary">配置门户超级管理员</h1>
          <p className="mt-4 text-sm leading-6 text-text-secondary">
            当前门户还没有可管理平台的超级管理员。完成配置后，这个账号将拥有用户、租户、系统入口和权限配置的最高管理权限。
          </p>
        </section>

        <section className="flex-1 rounded-radius-lg border border-border-subtle bg-bg-secondary p-8">
          <form onSubmit={handleSubmit} className="grid grid-cols-2 gap-5">
            <div className="col-span-2 space-y-1.5">
              <label className="text-sm font-medium text-text-secondary">默认租户名称</label>
              <Input value={tenantName} onChange={(e) => setTenantName(e.target.value)} />
            </div>
            <div className="space-y-1.5">
              <label className="text-sm font-medium text-text-secondary">管理员账号</label>
              <Input value={username} onChange={(e) => setUsername(e.target.value)} autoComplete="username" />
            </div>
            <div className="space-y-1.5">
              <label className="text-sm font-medium text-text-secondary">显示名称</label>
              <Input value={displayName} onChange={(e) => setDisplayName(e.target.value)} />
            </div>
            <div className="col-span-2 space-y-1.5">
              <label className="text-sm font-medium text-text-secondary">邮箱</label>
              <Input value={email} onChange={(e) => setEmail(e.target.value)} type="email" />
            </div>
            <div className="space-y-1.5">
              <label className="text-sm font-medium text-text-secondary">密码</label>
              <Input
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                type="password"
                autoComplete="new-password"
              />
            </div>
            <div className="space-y-1.5">
              <label className="text-sm font-medium text-text-secondary">确认密码</label>
              <Input
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                type="password"
                autoComplete="new-password"
              />
            </div>
            {error && <p className="col-span-2 text-sm text-color-error">{error}</p>}
            <div className="col-span-2 flex justify-end border-t border-border-faint pt-5">
              <Button type="submit" disabled={loading}>
                {loading ? "正在初始化..." : "完成初始化"}
              </Button>
            </div>
          </form>
        </section>
      </div>
    </main>
  );
}
