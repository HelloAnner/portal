import "next/server";

declare module "next/server" {
  interface NextRequest {
    /** Client IP address (may be provided by the runtime). */
    ip?: string | null;
  }
}
