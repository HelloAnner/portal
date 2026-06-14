"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { fetchApi, handleApiError } from "@/lib/api-client";

interface TenantOption {
  id: string;
  code: string;
  name: string;
}

interface ProfilePreferences {
  homeGroupMode?: string;
  showPermissionRequest?: boolean;
  [key: string]: unknown;
}

interface ProfileEditorProps {
  displayName: string;
  avatarUrl: string | null;
  defaultTenantId: string | null;
  preferences: ProfilePreferences;
  tenants: TenantOption[];
  onSaved?: () => void;
}

interface AvatarResponse {
  avatarUrl: string;
}

export function ProfileEditor({
  displayName,
  avatarUrl,
  defaultTenantId,
  preferences,
  tenants,
  onSaved,
}: ProfileEditorProps) {
  const [formDisplayName, setFormDisplayName] = useState(displayName);
  const [formAvatarUrl, setFormAvatarUrl] = useState(avatarUrl || "");
  const [formTenantId, setFormTenantId] = useState(defaultTenantId || "");
  const [homeGroupMode, setHomeGroupMode] = useState(preferences.homeGroupMode || "frequent_first");
  const [showPermissionRequest, setShowPermissionRequest] = useState(
    preferences.showPermissionRequest !== false
  );
  const [uploading, setUploading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState("");

  useEffect(() => {
    setFormDisplayName(displayName);
    setFormAvatarUrl(avatarUrl || "");
    setFormTenantId(defaultTenantId || "");
    setHomeGroupMode(preferences.homeGroupMode || "frequent_first");
    setShowPermissionRequest(preferences.showPermissionRequest !== false);
  }, [displayName, avatarUrl, defaultTenantId, preferences]);

  async function handleAvatarFileChange(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;

    setUploading(true);
    setMessage("");
    try {
      const base64 = await new Promise<string>((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve(reader.result as string);
        reader.onerror = reject;
        reader.readAsDataURL(file);
      });

      const data = await fetchApi<AvatarResponse>("/api/profile/avatar", {
        method: "POST",
        body: JSON.stringify({ base64 }),
      });
      setFormAvatarUrl(data.avatarUrl);
      setMessage("头像已更新");
      onSaved?.();
    } catch (err: unknown) {
      const { message } = handleApiError(err);
      setMessage(message || "头像上传失败");
    } finally {
      setUploading(false);
    }
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setMessage("");

    const payload: Record<string, unknown> = {
      displayName: formDisplayName.trim(),
      preferences: {
        ...preferences,
        homeGroupMode,
        showPermissionRequest,
      },
    };
    if (formAvatarUrl.trim()) {
      payload.avatarUrl = formAvatarUrl.trim();
    } else {
      payload.avatarUrl = null;
    }
    if (formTenantId) {
      payload.defaultTenantId = formTenantId;
    } else {
      payload.defaultTenantId = null;
    }

    try {
      await fetchApi<unknown>("/api/profile", {
        method: "PATCH",
        body: JSON.stringify(payload),
      });
      setMessage("资料已保存");
      onSaved?.();
    } catch (err: unknown) {
      const { message } = handleApiError(err);
      setMessage(message || "保存失败");
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="rounded-radius-md border border-border-subtle bg-bg-secondary p-6">
      <h2 className="mb-5 text-base font-semibold text-text-primary">编辑资料</h2>

      <form onSubmit={handleSubmit} className="space-y-5">
        <div className="space-y-1.5">
          <label className="text-sm font-medium text-text-secondary">显示名</label>
          <Input
            value={formDisplayName}
            onChange={(e) => setFormDisplayName(e.target.value)}
            placeholder="请输入显示名"
            required
          />
        </div>

        <div className="space-y-1.5">
          <label className="text-sm font-medium text-text-secondary">头像</label>
          <div className="flex items-center gap-4">
            <div className="flex h-14 w-14 items-center justify-center overflow-hidden rounded-full bg-bg-tertiary">
              {formAvatarUrl ? (
                <img src={formAvatarUrl} alt="avatar" className="h-full w-full object-cover" />
              ) : (
                <span className="text-lg text-text-primary">{formDisplayName.slice(0, 1)}</span>
              )}
            </div>
            <label className="cursor-pointer">
              <Button type="button" variant="secondary" disabled={uploading} asChild>
                <span>{uploading ? "上传中..." : "更换头像"}</span>
              </Button>
              <input
                type="file"
                accept="image/*"
                className="hidden"
                onChange={handleAvatarFileChange}
              />
            </label>
          </div>
          <Input
            className="mt-2"
            value={formAvatarUrl}
            onChange={(e) => setFormAvatarUrl(e.target.value)}
            placeholder="或输入头像 URL"
          />
        </div>

        <div className="space-y-1.5">
          <label className="text-sm font-medium text-text-secondary">默认租户</label>
          <select
            value={formTenantId}
            onChange={(e) => setFormTenantId(e.target.value)}
            className="flex h-10 w-full rounded-radius-sm border border-border-subtle bg-bg-secondary px-3 py-2 text-sm text-text-primary focus-visible:outline-none focus-visible:border-text-tertiary"
          >
            <option value="">未选择</option>
            {tenants.map((t) => (
              <option key={t.id} value={t.id}>
                {t.name}
              </option>
            ))}
          </select>
        </div>

        <div className="space-y-1.5">
          <label className="text-sm font-medium text-text-secondary">首页分组模式</label>
          <select
            value={homeGroupMode}
            onChange={(e) => setHomeGroupMode(e.target.value)}
            className="flex h-10 w-full rounded-radius-sm border border-border-subtle bg-bg-secondary px-3 py-2 text-sm text-text-primary focus-visible:outline-none focus-visible:border-text-tertiary"
          >
            <option value="frequent_first">常用优先</option>
            <option value="all_first">全部优先</option>
          </select>
        </div>

        <div className="flex items-center gap-2">
          <input
            id="showPermissionRequest"
            type="checkbox"
            checked={showPermissionRequest}
            onChange={(e) => setShowPermissionRequest(e.target.checked)}
            className="h-4 w-4 rounded border-border-subtle"
          />
          <label htmlFor="showPermissionRequest" className="text-sm text-text-secondary">
            显示权限申请入口
          </label>
        </div>

        {message && (
          <p
            className={`text-sm ${message.includes("失败") || message.includes("错误") ? "text-color-error" : "text-color-success"}`}
          >
            {message}
          </p>
        )}

        <div className="flex justify-end pt-2">
          <Button type="submit" disabled={saving}>
            {saving ? "保存中..." : "保存资料"}
          </Button>
        </div>
      </form>
    </div>
  );
}
