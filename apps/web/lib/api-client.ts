import { env } from "./env";

export type ApiErrorCode =
  | "AUTH_REQUIRED"
  | "PERMISSION_DENIED"
  | "SYSTEM_DISABLED"
  | "TENANT_DISABLED"
  | "INVALID_SUBSYSTEM_TICKET"
  | "VALIDATION_FAILED"
  | "CONFLICT"
  | "NOT_FOUND"
  | "INTERNAL_ERROR";

export interface ApiErrorBody {
  code: ApiErrorCode;
  message: string;
}

export interface ApiResponse<T> {
  data: T | null;
  requestId: string;
  error: ApiErrorBody | null;
}

export class ApiError extends Error {
  constructor(
    public code: ApiErrorCode,
    message: string,
    public requestId?: string
  ) {
    super(message);
    this.name = "ApiError";
  }
}

function getBaseUrl(): string {
  if (typeof window !== "undefined") {
    return process.env.NEXT_PUBLIC_API_URL || "";
  }
  return env.APP_BASE_URL;
}

export async function fetchApi<T>(
  path: string,
  options: RequestInit = {}
): Promise<T> {
  const baseUrl = getBaseUrl();
  const url = `${baseUrl}${path}`;
  const res = await fetch(url, {
    ...options,
    credentials: "include",
    headers: {
      "Content-Type": "application/json",
      ...options.headers,
    },
  });

  const json = (await res.json()) as ApiResponse<T>;

  if (json.error) {
    throw new ApiError(json.error.code, json.error.message, json.requestId);
  }

  if (json.data === null || json.data === undefined) {
    throw new ApiError("INTERNAL_ERROR", "Invalid API response", json.requestId);
  }

  return json.data;
}

export function handleApiError(error: unknown): { code: ApiErrorCode; message: string } {
  if (error instanceof ApiError) {
    return { code: error.code, message: error.message };
  }
  if (error instanceof Error) {
    return { code: "INTERNAL_ERROR", message: error.message };
  }
  return { code: "INTERNAL_ERROR", message: "Unknown error" };
}

export function isAuthError(error: unknown): boolean {
  return error instanceof ApiError && error.code === "AUTH_REQUIRED";
}

export function isPermissionError(error: unknown): boolean {
  return error instanceof ApiError && error.code === "PERMISSION_DENIED";
}
