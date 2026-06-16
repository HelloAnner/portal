"use client";

import { useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { fetchApi, handleApiError } from "@/lib/api-client";

interface EnterResponse {
  callbackUrl: string;
  code: string;
}

interface SystemCardProps {
  systemCode: string;
  name: string;
  description: string | null;
  iconUrl: string | null;
  status: string;
  identityLabel: string;
  tenantLabel: string;
  permissionSummary: string[];
  enterable: boolean;
}

export function SystemCard({
  systemCode,
  name,
  description,
  identityLabel,
  tenantLabel,
  permissionSummary,
  enterable,
}: SystemCardProps) {
  const [loading, setLoading] = useState(false);

  async function enter() {
    if (!enterable) return;
    setLoading(true);
    try {
      const data = await fetchApi<EnterResponse>(`/api/portal/systems/${systemCode}/enter`, {
        method: "POST",
        body: JSON.stringify({}),
      });
      if (data.callbackUrl) {
        const url = new URL(data.callbackUrl);
        url.searchParams.set("code", data.code);
        const opened = window.open(url.toString(), "_blank");
        if (opened) {
          opened.opener = null;
        }
        if (!opened) {
          window.location.href = url.toString();
        }
      } else {
        alert("进入失败");
      }
    } catch (err: unknown) {
      const { message } = handleApiError(err);
      alert(message || "进入失败");
    } finally {
      setLoading(false);
    }
  }

  const statusText =
    status === "maintenance" ? "维护中" : status === "onboarding" ? "接入中" : "";

  return (
    <div className="flex flex-col justify-between rounded-radius-md border border-border-subtle bg-bg-secondary p-5">
      <div>
        <div className="flex items-start justify-between gap-3">
          <h3 className="text-base font-semibold text-text-primary">{name}</h3>
          <Badge variant="identity">{identityLabel}</Badge>
        </div>
        {description && <p className="mt-2 text-sm text-text-muted">{description}</p>}
        <div className="mt-3 flex flex-wrap gap-2">
          <Badge variant="default">{tenantLabel}</Badge>
          {permissionSummary.map((p) => (
            <Badge key={p} variant="default">
              {p}
            </Badge>
          ))}
          {statusText && <Badge variant="warning">{statusText}</Badge>}
        </div>
      </div>
      <div className="mt-5">
        <Button
          variant="primary"
          className="w-full"
          disabled={!enterable || loading}
          onClick={enter}
        >
          {loading ? "跳转中..." : enterable ? "进入系统" : "不可进入"}
        </Button>
      </div>
    </div>
  );
}
