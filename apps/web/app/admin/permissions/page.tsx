"use client";

import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Drawer } from "@/components/admin/drawer";
import { SelectField } from "@/components/admin/select-field";
import { Textarea } from "@/components/admin/textarea";
import { fetchApi, handleApiError, isAuthError, isPermissionError } from "@/lib/api-client";

interface SystemCol {
  id: string;
  code: string;
  name: string;
}

interface TenantCol {
  id: string;
  code: string;
  name: string;
}

interface Subject {
  id: string;
  code?: string;
  name: string;
  status?: string;
}

interface MatrixCell {
  assignmentId?: string;
  visible: boolean;
  accessible: boolean;
  permissions: string[];
  systemRoles: string[];
  scopes: Array<{ scopeType: string; scopeCode: string }>;
}

interface MatrixRow {
  subject: Subject;
  cells: MatrixCell[];
}

interface MatrixData {
  view: "user" | "tenant" | "system" | "role";
  tenantId: string | null;
  systems: SystemCol[];
  tenants: TenantCol[];
  subjects: Subject[];
  rows: MatrixRow[];
}

const views = [
  { key: "user", label: "用户视角" },
  { key: "tenant", label: "租户视角" },
  { key: "system", label: "系统视角" },
  { key: "role", label: "角色视角" },
] as const;

function handleRedirect(err: unknown) {
  if (isAuthError(err)) {
    window.location.href = "/login";
    return true;
  }
  if (isPermissionError(err)) {
    window.location.href = "/no-permission";
    return true;
  }
  return false;
}

export default function PermissionsPage() {
  const [view, setView] = useState<MatrixData["view"]>("user");
  const [tenantId, setTenantId] = useState<string>("");
  const [data, setData] = useState<MatrixData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [saveKey, setSaveKey] = useState<string | null>(null);
  const [editCell, setEditCell] = useState<{ row: MatrixRow; colIndex: number } | null>(null);

  useEffect(() => {
    const params = new URLSearchParams({ view });
    if (tenantId && (view === "user" || view === "role")) params.set("tenantId", tenantId);
    setLoading(true);
    setError("");
    fetchApi<MatrixData>(`/api/admin/permissions/matrix?${params.toString()}`)
      .then((matrix) => {
        setData(matrix);
        setError("");
        if (matrix.tenantId && !tenantId) setTenantId(matrix.tenantId);
      })
      .catch((err: unknown) => {
        if (handleRedirect(err)) return;
        const { message } = handleApiError(err);
        setError(message);
      })
      .finally(() => setLoading(false));
  }, [view, tenantId, setData, setError, setTenantId]);

  const columns = useMemo(() => {
    if (!data) return [];
    return view === "system" ? data.tenants : data.systems;
  }, [data, view]);

  function buildAssignment(row: MatrixRow, colIndex: number, partial: Partial<MatrixCell>) {
    if (!data) return null;
    const cell = row.cells[colIndex];
    const base = {
      id: cell.assignmentId,
      visible: partial.visible ?? cell.visible,
      accessible: partial.accessible ?? cell.accessible,
      permissions: partial.permissions ?? cell.permissions,
      systemRoles: partial.systemRoles ?? cell.systemRoles,
      scopes: partial.scopes ?? cell.scopes,
      sourceNote: "",
    };

    if (view === "user") {
      return {
        ...base,
        subjectType: "user" as const,
        subjectId: row.subject.id,
        tenantId,
        systemId: data.systems[colIndex].id,
      };
    }
    if (view === "role") {
      return {
        ...base,
        subjectType: "role" as const,
        subjectId: row.subject.id,
        tenantId,
        systemId: data.systems[colIndex].id,
      };
    }
    if (view === "tenant") {
      return {
        ...base,
        subjectType: "tenant" as const,
        subjectId: row.subject.id,
        tenantId: row.subject.id,
        systemId: data.systems[colIndex].id,
      };
    }
    return {
      ...base,
      subjectType: "tenant" as const,
      subjectId: data.tenants[colIndex].id,
      tenantId: data.tenants[colIndex].id,
      systemId: row.subject.id,
    };
  }

  async function saveCell(row: MatrixRow, colIndex: number, partial: Partial<MatrixCell>) {
    const assignment = buildAssignment(row, colIndex, partial);
    if (!assignment) return;
    const key = `${row.subject.id}:${colIndex}`;
    setSaveKey(key);
    try {
      await fetchApi<unknown>("/api/admin/permissions/assignments", {
        method: "PUT",
        body: JSON.stringify({ assignments: [assignment] }),
      });
      const next = { ...row.cells[colIndex], ...partial };
      row.cells[colIndex] = next;
      setData({ ...data! });
    } catch (err: unknown) {
      if (handleRedirect(err)) return;
      const { message } = handleApiError(err);
      alert(message);
    } finally {
      setSaveKey(null);
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <h1 className="text-xl font-semibold text-text-primary">权限矩阵</h1>
        <div className="flex flex-wrap items-center gap-2">
          {views.map((v) => (
            <Button
              key={v.key}
              variant={view === v.key ? "primary" : "secondary"}
              onClick={() => setView(v.key as MatrixData["view"])}
            >
              {v.label}
            </Button>
          ))}
        </div>
      </div>

      {data && (view === "user" || view === "role") && (
        <div className="w-full sm:w-64">
          <SelectField
            label="租户上下文"
            value={tenantId}
            onChange={(e) => setTenantId(e.target.value)}
            options={data.tenants.map((t) => ({ value: t.id, label: t.name }))}
          />
        </div>
      )}

      {loading && <p className="text-text-muted">加载中...</p>}
      {error && <p className="text-color-error">{error}</p>}

      {data && (
        <div className="overflow-auto rounded-radius-md border border-border-subtle">
          <table className="w-full text-left text-sm">
            <thead className="bg-bg-tertiary text-text-secondary">
              <tr>
                <th className="sticky left-0 z-10 bg-bg-tertiary px-3 py-2 font-medium">主体</th>
                {columns.map((c) => (
                  <th key={c.id} className="min-w-[160px] px-3 py-2 font-medium">
                    {c.name}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-border-subtle">
              {data.rows.map((row) => (
                <tr key={row.subject.id} className="hover:bg-hover-bg">
                  <td className="sticky left-0 z-10 bg-bg-secondary px-3 py-2 font-medium text-text-primary">
                    {row.subject.name}
                  </td>
                  {row.cells.map((cell, idx) => {
                    const key = `${row.subject.id}:${idx}`;
                    return (
                      <td key={key} className="px-3 py-2 align-top">
                        <button
                          onClick={() => setEditCell({ row, colIndex: idx })}
                          disabled={saveKey === key}
                          className="w-full text-left"
                        >
                          <div className="flex flex-wrap items-center gap-2">
                            <label
                              className="flex items-center gap-1 text-xs text-text-secondary"
                              onClick={(e) => e.stopPropagation()}
                            >
                              <input
                                type="checkbox"
                                checked={cell.visible}
                                onChange={() => saveCell(row, idx, { visible: !cell.visible })}
                                disabled={saveKey === key}
                              />
                              可见
                            </label>
                            <label
                              className="flex items-center gap-1 text-xs text-text-secondary"
                              onClick={(e) => e.stopPropagation()}
                            >
                              <input
                                type="checkbox"
                                checked={cell.accessible}
                                onChange={() => saveCell(row, idx, { accessible: !cell.accessible })}
                                disabled={saveKey === key}
                              />
                              可访问
                            </label>
                          </div>
                          <div className="mt-1 flex flex-wrap gap-1">
                            {cell.systemRoles.slice(0, 2).map((r) => (
                              <Badge key={r} variant="identity">
                                {r}
                              </Badge>
                            ))}
                            {cell.permissions.slice(0, 2).map((p) => (
                              <Badge key={p} variant="default">
                                {p}
                              </Badge>
                            ))}
                            {cell.systemRoles.length + cell.permissions.length > 2 && (
                              <Badge variant="default">
                                +{cell.systemRoles.length + cell.permissions.length - 2}
                              </Badge>
                            )}
                          </div>
                        </button>
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {editCell && data && (
        <EditDrawer
          key={`${editCell.row.subject.id}:${editCell.colIndex}`}
          editCell={editCell}
          data={data}
          onClose={() => setEditCell(null)}
          onSave={saveCell}
        />
      )}
    </div>
  );
}

interface EditDrawerProps {
  editCell: { row: MatrixRow; colIndex: number };
  data: MatrixData;
  onClose: () => void;
  onSave: (row: MatrixRow, colIndex: number, partial: Partial<MatrixCell>) => Promise<void>;
}

function EditDrawer({ editCell, data, onClose, onSave }: EditDrawerProps) {
  const { row, colIndex } = editCell;
  const cell = row.cells[colIndex];
  const colName = data.view === "system" ? data.tenants[colIndex].name : data.systems[colIndex].name;
  const [visible, setVisible] = useState(cell.visible);
  const [accessible, setAccessible] = useState(cell.accessible);
  const [permissions, setPermissions] = useState(cell.permissions.join(", "));
  const [systemRoles, setSystemRoles] = useState(cell.systemRoles.join(", "));
  const [scopes, setScopes] = useState(JSON.stringify(cell.scopes, null, 2));
  const [submitting, setSubmitting] = useState(false);

  async function submit() {
    setSubmitting(true);
    let parsedScopes: Array<{ scopeType: string; scopeCode: string }> = [];
    try {
      parsedScopes = scopes ? JSON.parse(scopes) : [];
    } catch {
      alert("scopes JSON 格式错误");
      setSubmitting(false);
      return;
    }
    await onSave(row, colIndex, {
      visible,
      accessible,
      permissions: permissions.split(",").map((s) => s.trim()).filter(Boolean),
      systemRoles: systemRoles.split(",").map((s) => s.trim()).filter(Boolean),
      scopes: parsedScopes,
    });
    onClose();
    setSubmitting(false);
  }

  return (
    <Drawer
      open
      onClose={onClose}
      title="编辑权限"
      footer={
        <div className="flex justify-end gap-2">
          <Button variant="secondary" onClick={onClose}>
            取消
          </Button>
          <Button onClick={submit} disabled={submitting}>
            保存
          </Button>
        </div>
      }
    >
      <div className="space-y-4">
        <div className="text-sm text-text-secondary">
          <p>主体：{row.subject.name}</p>
          <p>目标：{colName}</p>
        </div>
        <div className="flex items-center gap-4">
          <label className="flex items-center gap-2 text-sm text-text-primary">
            <input type="checkbox" checked={visible} onChange={(e) => setVisible(e.target.checked)} />
            可见
          </label>
          <label className="flex items-center gap-2 text-sm text-text-primary">
            <input type="checkbox" checked={accessible} onChange={(e) => setAccessible(e.target.checked)} />
            可访问
          </label>
        </div>
        <div>
          <label className="text-sm font-medium text-text-secondary">权限列表（逗号分隔）</label>
          <Input value={permissions} onChange={(e) => setPermissions(e.target.value)} />
        </div>
        <div>
          <label className="text-sm font-medium text-text-secondary">系统角色（逗号分隔）</label>
          <Input value={systemRoles} onChange={(e) => setSystemRoles(e.target.value)} />
        </div>
        <div>
          <label className="text-sm font-medium text-text-secondary">管理范围（JSON）</label>
          <Textarea value={scopes} onChange={(e) => setScopes(e.target.value)} />
        </div>
      </div>
    </Drawer>
  );
}
