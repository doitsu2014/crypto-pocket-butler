import type { Metadata } from "next";
import "./globals.css";
import { ToastProvider } from "@/contexts/ToastContext";
import ToastContainer from "@/components/Toast";
import { QueryClientProvider } from "@/contexts/QueryClientProvider";
import SessionProviderWrapper from "@/components/SessionProviderWrapper";

export const metadata: Metadata = {
  title: "Crypto Pocket Butler",
  description: "Crypto portfolio management with intelligent rebalancing",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="antialiased">
        <SessionProviderWrapper>
          <QueryClientProvider>
            <ToastProvider>
              {children}
              <ToastContainer />
            </ToastProvider>
          </QueryClientProvider>
        </SessionProviderWrapper>
      </body>
    </html>
  );
}
