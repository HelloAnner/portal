import Link from "next/link";
import { redirect } from "next/navigation";
import { cookies } from "next/headers";
import { fetchApi } from "@/lib/api-client";
import { AdminSidebar } from "@/components/admin/sidebar";

interface MeData {
  canEnterAdmin: boolean;
  displayName?: string;
  isSuperAdmin?: boolean;
}

export default async function AdminLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  let me: MeData;
  try {
    const cookieHeader = cookies().toString();
    me = await fetchApi<MeData>("/api/auth/me", {
      headers: { Cookie: cookieHeader },
    });
    if (!me.canEnterAdmin) {
      redirect("/no-permission");
    }
  } catch {
    redirect("/login?redirectTo=/admin");
  }

  return (
    <div className="flex min-h-screen bg-bg-primary">
      <AdminSidebar
        showAdmin
        displayName={me.displayName || "Portal Admin"}
        roleLabel={me.isSuperAdmin ? "Super Admin" : "Admin"}
      />
      <div className="flex min-w-0 flex-1 flex-col overflow-hidden">
        <header className="flex h-16 items-center justify-between bg-bg-primary px-8">
          <span className="text-sm font-semibold text-text-primary">Portal Admin</span>
          <Link href="/" className="text-sm text-text-secondary hover:text-text-primary">
            返回门户首页
          </Link>
        </header>
        <main className="flex-1 overflow-auto px-8 pb-8">{children}</main>
      </div>
    </div>
  );
}
