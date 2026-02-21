export { auth as middleware } from "@/auth";

export const config = {
  matcher: ["/dashboard/:path*", "/portfolios/:path*", "/api/backend/:path*", "/admin/:path*"],
};
