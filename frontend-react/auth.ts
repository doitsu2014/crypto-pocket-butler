import NextAuth from "next-auth";
import Keycloak from "next-auth/providers/keycloak";

/**
 * Refresh the access token using the refresh token
 */
async function refreshAccessToken(token: any) {
  try {
    const response = await fetch(`${process.env.KEYCLOAK_ISSUER}/protocol/openid-connect/token`, {
      method: "POST",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
      },
      body: new URLSearchParams({
        client_id: process.env.KEYCLOAK_CLIENT_ID!,
        client_secret: process.env.KEYCLOAK_CLIENT_SECRET!,
        grant_type: "refresh_token",
        refresh_token: token.refreshToken,
      }),
    });

    const refreshedTokens = await response.json();

    if (!response.ok) {
      throw refreshedTokens;
    }

    return {
      ...token,
      accessToken: refreshedTokens.access_token,
      expiresAt: Math.floor(Date.now() / 1000) + refreshedTokens.expires_in,
      refreshToken: refreshedTokens.refresh_token ?? token.refreshToken, // Fall back to old refresh token
      idToken: refreshedTokens.id_token ?? token.idToken,
    };
  } catch (error) {
    console.error("Error refreshing access token", error);
    return {
      ...token,
      error: "RefreshAccessTokenError",
    };
  }
}

export const { handlers, signIn, signOut, auth } = NextAuth({
  providers: [
    Keycloak({
      clientId: process.env.KEYCLOAK_CLIENT_ID!,
      clientSecret: process.env.KEYCLOAK_CLIENT_SECRET!,
      issuer: process.env.KEYCLOAK_ISSUER!,
      authorization: {
        params: {
          // PKCE is automatically enabled by NextAuth.js v5
          // offline_access scope enables refresh tokens
          scope: "openid profile email offline_access",
        },
      },
    }),
  ],
  callbacks: {
    async jwt({ token, account, trigger }) {
      // Initial sign in - persist OAuth tokens
      if (account) {
        token.accessToken = account.access_token;
        token.idToken = account.id_token;
        token.refreshToken = account.refresh_token;
        token.expiresAt = account.expires_at;
        token.issuedAt = Math.floor(Date.now() / 1000);
        return token;
      }

      // Return previous token if the access token has not expired yet
      // Check if token has lived more than 50% of its lifetime
      const now = Math.floor(Date.now() / 1000);
      const expiresAt = (token.expiresAt as number) || 0;
      const issuedAt = (token.issuedAt as number) || now;
      const tokenLifetime = expiresAt - issuedAt;
      const tokenAge = now - issuedAt;
      
      // Refresh if token has lived more than 50% of its lifetime
      const shouldRefresh = tokenAge > (tokenLifetime * 0.5);

      if (!shouldRefresh) {
        return token;
      }

      // Token has passed 50% of its lifetime, refresh it
      console.log("Token has passed 50% of lifetime, refreshing...");
      return refreshAccessToken(token);
    },
    async session({ session, token }) {
      // Send properties to the client, like an access_token from a provider
      session.accessToken = token.accessToken as string;
      session.idToken = token.idToken as string;
      session.error = token.error as string | undefined;
      return session;
    },
  },
  pages: {
    signIn: "/auth/signin",
  },
  session: {
    strategy: "jwt",
  },
});
