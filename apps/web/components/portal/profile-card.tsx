"use client";

import { Badge } from "@/components/ui/badge";

interface ProfileCardProps {
  username: string;
  displayName: string;
  email: string | null;
  phone: string | null;
  organizationPath: string | null;
  status: string;
  avatarUrl: string | null;
}

function statusLabel(status: string): string {
  switch (status) {
    case "active":
      return "正常";
    case "disabled":
      return "已禁用";
    case "pending":
      return "待激活";
    case "archived":
      return "已归档";
    default:
      return status;
  }
}

function statusVariant(status: string): "success" | "error" | "warning" | "default" {
  switch (status) {
    case "active":
      return "success";
    case "disabled":
    case "archived":
      return "error";
    case "pending":
      return "warning";
    default:
      return "default";
  }
}

export function ProfileCard({
  username,
  displayName,
  email,
  phone,
  organizationPath,
  status,
  avatarUrl,
}: ProfileCardProps) {
  return (
    <div className="rounded-radius-md border border-border-subtle bg-bg-secondary p-6">
      <div className="flex flex-col items-center text-center">
        <div className="flex h-24 w-24 items-center justify-center overflow-hidden rounded-full bg-bg-tertiary">
          {avatarUrl ? (
            <img src={avatarUrl} alt={displayName} className="h-full w-full object-cover" />
          ) : (
            <span className="text-2xl font-semibold text-text-primary">
              {displayName.slice(0, 1)}
            </span>
          )}
        </div>
        <h2 className="mt-4 text-lg font-semibold text-text-primary">{displayName}</h2>
        <p className="text-sm text-text-muted">{username}</p>
        <div className="mt-3">
          <Badge variant={statusVariant(status)}>{statusLabel(status)}</Badge>
        </div>
      </div>

      <div className="mt-6 space-y-3 border-t border-border-faint pt-5 text-sm">
        {email && (
          <div className="flex justify-between">
            <span className="text-text-muted">邮箱</span>
            <span className="text-text-secondary">{email}</span>
          </div>
        )}
        {phone && (
          <div className="flex justify-between">
            <span className="text-text-muted">手机</span>
            <span className="text-text-secondary">{phone}</span>
          </div>
        )}
        {organizationPath && (
          <div className="flex justify-between">
            <span className="text-text-muted">组织路径</span>
            <span className="text-text-secondary">{organizationPath}</span>
          </div>
        )}
      </div>
    </div>
  );
}
