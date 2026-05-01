import { auth } from "@/auth";
import { redirect } from "next/navigation";
import PortfoliosClient from "./components/PortfoliosClient";
import AppLayout from "@/components/AppLayout";

export default async function PortfoliosPage() {
  const session = await auth();

  if (!session?.user) {
    redirect("/auth/signin");
  }

  return (
    <AppLayout userEmail={session.user.email}>
      <PortfoliosClient />
    </AppLayout>
  );
}
