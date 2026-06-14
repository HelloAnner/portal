import Link from "next/link";
import { redirect } from "next/navigation";
import { cookies } from "next/headers";
import { fetchApi } from "@/lib/api-client";
import { AdminSidebar } from "@/components/admin/sidebar";

interface MeData {
  canEnterAdmin: boolean;
}

export default async function AdminLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  try {
    const cookieHeader = cookies().toString();
    const me = await fetchApi<MeData>("/api/auth/me", {
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
      <AdminSidebar />
      <div className="flex flex-1 flex-col overflow-hidden">
        <header className="flex h-[60px] items-center justify-between border-b border-border-subtle bg-bg-secondary px-6">
          <span className="text-base font-semibold text-text-primary">管理后台</span>
          <Link href="/" className="text-sm text-text-secondary hover:text-text-primary">
            返回门户首页
          </Link>
        </header>
        <main className="flex-1 overflow-auto p-6">{children}</main>
      </div>
    </div>
  );
}
