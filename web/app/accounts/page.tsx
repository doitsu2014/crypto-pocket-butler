import { auth } from "@/auth";
import { redirect } from "next/navigation";
import AccountsClient from "./components/AccountsClient";
import AppLayout from "@/components/AppLayout";

export default async function AccountsPage() {
  const session = await auth();

  if (!session?.user) {
    redirect("/auth/signin");
  }

  return (
    <AppLayout userEmail={session.user.email}>
      <AccountsClient />
    </AppLayout>
  );
}
