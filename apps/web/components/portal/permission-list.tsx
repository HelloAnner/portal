"use client";

import { Badge } from "@/components/ui/badge";

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

interface PermissionListProps {
  portalRoles: string[];
  tenants: TenantItem[];
  systems: SystemPermissionItem[];
  onSelectSystem: (systemCode: string) => void;
}

function statusText(status: string): string {
  switch (status) {
    case "maintenance":
      return "维护中";
    case "onboarding":
      return "接入中";
    default:
      return "";
  }
}

export function PermissionList({
  portalRoles,
  tenants,
  systems,
  onSelectSystem,
}: PermissionListProps) {
  return (
    <div className="space-y-6">
      <section>
        <h3 className="mb-3 text-sm font-medium uppercase tracking-wide text-text-muted">
          门户角色
        </h3>
        {portalRoles.length === 0 ? (
          <p className="text-sm text-text-muted">暂无门户角色</p>
        ) : (
          <div className="flex flex-wrap gap-2">
            {portalRoles.map((role) => (
              <Badge key={role} variant="identity">
                {role}
              </Badge>
            ))}
          </div>
        )}
      </section>

      <section>
        <h3 className="mb-3 text-sm font-medium uppercase tracking-wide text-text-muted">
          所属租户
        </h3>
        {tenants.length === 0 ? (
          <p className="text-sm text-text-muted">未加入任何租户</p>
        ) : (
          <div className="flex flex-wrap gap-2">
            {tenants.map((tenant) => (
              <Badge key={tenant.id} variant="default">
                {tenant.name}
              </Badge>
            ))}
          </div>
        )}
      </section>

      <section>
        <h3 className="mb-3 text-sm font-medium uppercase tracking-wide text-text-muted">
          可访问系统
        </h3>
        {systems.length === 0 ? (
          <div className="rounded-radius-md border border-border-subtle bg-bg-secondary p-8 text-center">
            <p className="text-sm text-text-muted">暂无可访问系统</p>
          </div>
        ) : (
          <div className="overflow-hidden rounded-radius-md border border-border-subtle bg-bg-secondary">
            <div className="grid grid-cols-12 gap-3 border-b border-border-faint bg-bg-tertiary px-4 py-2.5 text-xs font-medium text-text-muted">
              <div className="col-span-3">系统</div>
              <div className="col-span-2">租户</div>
              <div className="col-span-2">身份</div>
              <div className="col-span-2">来源</div>
              <div className="col-span-3">范围</div>
            </div>
            <div className="divide-y divide-border-faint">
              {systems.map((s) => (
                <button
                  key={`${s.systemCode}-${s.tenantId}`}
                  onClick={() => onSelectSystem(s.systemCode)}
                  className="grid w-full grid-cols-12 gap-3 px-4 py-3 text-left transition-colors hover:bg-hover-bg"
                >
                  <div className="col-span-3">
                    <p className="text-sm font-medium text-text-primary">{s.name}</p>
                    <p className="text-xs text-text-muted">{s.systemCode}</p>
                  </div>
                  <div className="col-span-2">
                    <span className="text-sm text-text-secondary">{s.tenantName}</span>
                  </div>
                  <div className="col-span-2">
                    <Badge variant="identity">{s.identity}</Badge>
                    {statusText(s.status) && (
                      <Badge variant="warning" className="ml-1.5">
                        {statusText(s.status)}
                      </Badge>
                    )}
                    {!s.accessible && (
                      <Badge variant="error" className="ml-1.5">
                        不可进
                      </Badge>
                    )}
                  </div>
                  <div className="col-span-2">
                    <div className="flex flex-wrap gap-1">
                      {s.sourceSummary.slice(0, 2).map((source) => (
                        <Badge key={source} variant="default">
                          {source}
                        </Badge>
                      ))}
                      {s.sourceSummary.length > 2 && (
                        <Badge variant="default">+{s.sourceSummary.length - 2}</Badge>
                      )}
                    </div>
                  </div>
                  <div className="col-span-3">
                    <div className="flex flex-wrap gap-1">
                      {s.scopeSummary.slice(0, 2).map((scope) => (
                        <Badge key={scope} variant="info">
                          {scope}
                        </Badge>
                      ))}
                      {s.scopeSummary.length > 2 && (
                        <Badge variant="info">+{s.scopeSummary.length - 2}</Badge>
                      )}
                    </div>
                  </div>
                </button>
              ))}
            </div>
          </div>
        )}
      </section>
    </div>
  );
}
