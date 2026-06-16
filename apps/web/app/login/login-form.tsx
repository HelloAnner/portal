"use client";

import { useEffect, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { fetchApi, isAuthError } from "@/lib/api-client";

interface LoginResponse {
  redirectTo: string;
}

interface SetupStatus {
  needsSetup: boolean;
}

export function LoginForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [rememberMe, setRememberMe] = useState(false);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    fetchApi<SetupStatus>("/api/setup/status")
      .then((status) => {
        if (status.needsSetup) {
          router.replace("/setup");
        }
      })
      .catch(() => {});
  }, [router]);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError("");
    setLoading(true);
    try {
      const data = await fetchApi<LoginResponse>("/api/auth/login", {
        method: "POST",
        body: JSON.stringify({
          username,
          password,
          rememberMe,
          redirectTo: searchParams.get("redirectTo") || "/",
        }),
      });
      router.push(data.redirectTo || "/");
      router.refresh();
    } catch (err: unknown) {
      setLoading(false);
      if (isAuthError(err)) {
        setError("账号或密码错误，或账号已被禁用");
      } else {
        setError("登录失败");
      }
    }
  }

  return (
    <main className="flex min-h-screen flex-col items-center justify-center bg-bg-primary p-6">
      <div className="mb-8 text-center">
        <h1 className="text-2xl font-semibold tracking-tight text-text-primary">企业门户</h1>
        <p className="mt-2 text-sm text-text-muted">企业内部系统的统一入口</p>
      </div>

      <div className="w-full max-w-sm rounded-radius-lg border border-border-subtle bg-bg-secondary p-8 shadow-none">
        <form onSubmit={handleSubmit} className="space-y-5">
          <div className="space-y-1.5">
            <label className="text-sm font-medium text-text-secondary">账号</label>
            <Input
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="请输入账号"
              autoComplete="username"
            />
          </div>
          <div className="space-y-1.5">
            <label className="text-sm font-medium text-text-secondary">密码</label>
            <Input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="请输入密码"
              autoComplete="current-password"
            />
          </div>
          <div className="flex items-center gap-2">
            <input
              id="remember"
              type="checkbox"
              checked={rememberMe}
              onChange={(e) => setRememberMe(e.target.checked)}
              className="h-4 w-4 rounded border-border-subtle text-text-primary"
            />
            <label htmlFor="remember" className="text-sm text-text-secondary">
              记住登录状态
            </label>
          </div>
          {error && <p className="text-sm text-color-error">{error}</p>}
          <Button type="submit" className="w-full" disabled={loading}>
            {loading ? "登录中..." : "登录"}
          </Button>
        </form>

        <div className="mt-6 border-t border-border-faint pt-5 text-center">
          <p className="text-xs text-text-muted">企业 SSO / LDAP / OAuth 扩展位</p>
        </div>
      </div>
    </main>
  );
}
