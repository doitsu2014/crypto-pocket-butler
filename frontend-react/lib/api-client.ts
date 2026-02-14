/**
 * Unified API Client for Backend Communication
 * 
 * This module provides a centralized, type-safe way to communicate with the backend API.
 * It handles authentication, error handling, and request/response formatting consistently.
 * 
 * @module lib/api-client
 * 
 * ## Architecture
 * 
 * Frontend → apiClient() → Next.js API Proxy → Backend
 * 
 * The Next.js API proxy (/app/api/backend/[...path]/route.ts) automatically:
 * - Extracts the access token from the NextAuth session
 * - Forwards requests to the backend with proper Authorization header
 * - Returns responses to the client
 * 
 * ## Usage Examples
 * 
 * ### GET Request (Authenticated)
 * ```typescript
 * const accounts = await apiClient<Account[]>("/v1/accounts");
 * ```
 * 
 * ### POST Request (Authenticated)
 * ```typescript
 * const newAccount = await apiClient<Account>("/v1/accounts", {
 *   method: "POST",
 *   body: { name: "My Wallet", type: "wallet", address: "0x..." }
 * });
 * ```
 * 
 * ### PUT Request (Authenticated)
 * ```typescript
 * const updated = await apiClient<Account>("/v1/accounts/123", {
 *   method: "PUT",
 *   body: { name: "Updated Name" }
 * });
 * ```
 * 
 * ### DELETE Request (Authenticated)
 * ```typescript
 * await apiClient<void>("/v1/accounts/123", {
 *   method: "DELETE"
 * });
 * ```
 * 
 * ## Error Handling
 * 
 * All errors are thrown as ApiError instances with:
 * - type: "auth" | "validation" | "server" | "network" | "unknown"
 * - statusCode: HTTP status code (if applicable)
 * - message: Human-readable error message
 * - details: Additional error information
 * 
 * Example:
 * ```typescript
 * try {
 *   await apiClient("/v1/accounts");
 * } catch (error) {
 *   if (error instanceof ApiError) {
 *     if (error.type === "auth") {
 *       // Redirect to login
 *     } else if (error.type === "validation") {
 *       // Show validation errors
 *     }
 *   }
 * }
 * ```
 * 
 * ## Direct Backend Client (Server-Side Only)
 * 
 * For server-side API routes or server components, use directBackendClient:
 * ```typescript
 * const data = await directBackendClient<Account[]>(
 *   "/v1/accounts",
 *   session.accessToken
 * );
 * ```
 */

const BACKEND_URL = process.env.NEXT_PUBLIC_BACKEND_URL || "http://localhost:3000";

export interface ApiClientOptions {
  method?: "GET" | "POST" | "PUT" | "DELETE" | "PATCH";
  body?: unknown;
  headers?: HeadersInit;
}

export type ApiErrorType = "network" | "auth" | "validation" | "server" | "unknown";

export class ApiError extends Error {
  public type: ApiErrorType;
  public statusCode?: number;
  public details?: unknown;

  constructor(message: string, type: ApiErrorType, statusCode?: number, details?: unknown) {
    super(message);
    this.name = "ApiError";
    this.type = type;
    this.statusCode = statusCode;
    this.details = details;
  }
}

function parseErrorResponse(response: Response, errorData: any): ApiError {
  const status = response.status;
  let message = errorData?.error || errorData?.message || response.statusText || "An error occurred";
  let type: ApiErrorType = "unknown";

  // Categorize errors by status code
  if (status === 401 || status === 403) {
    type = "auth";
    message = errorData?.error || "Authentication failed. Please sign in again.";
  } else if (status === 400 || status === 422) {
    type = "validation";
    message = errorData?.error || "Invalid input. Please check your data.";
  } else if (status >= 500) {
    type = "server";
    message = errorData?.error || "Server error. Please try again later.";
  } else if (status >= 400) {
    type = "unknown";
  }

  return new ApiError(message, type, status, errorData);
}

/**
 * Make an authenticated API call through the Next.js API proxy
 * This ensures the access token is securely handled server-side
 */
export async function apiClient<T>(
  endpoint: string,
  options: ApiClientOptions = {}
): Promise<T> {
  const { method = "GET", body, headers = {} } = options;

  const config: RequestInit = {
    method,
    headers: {
      "Content-Type": "application/json",
      ...headers,
    },
  };

  if (body) {
    config.body = JSON.stringify(body);
  }

  try {
    // Use the Next.js API route as a proxy to handle authentication
    const response = await fetch(`/api/backend${endpoint}`, config);

    if (!response.ok) {
      // Error responses often include JSON, but do not assume a body.
      const errorText = await response.text().catch(() => "");
      let errorData: any;
      try {
        errorData = errorText ? JSON.parse(errorText) : { error: response.statusText };
      } catch {
        errorData = { error: errorText || response.statusText };
      }
      throw parseErrorResponse(response, errorData);
    }

    // 204 No Content (common for DELETE)
    if (response.status === 204) {
      return undefined as unknown as T;
    }

    // Success responses may still have an empty body.
    const text = await response.text().catch(() => "");
    if (!text) {
      return undefined as unknown as T;
    }
    return JSON.parse(text) as T;
  } catch (error) {
    // Handle network errors
    if (error instanceof TypeError && error.message.includes("fetch")) {
      throw new ApiError("Network error. Please check your connection.", "network");
    }
    // Re-throw ApiError as-is
    if (error instanceof ApiError) {
      throw error;
    }
    // Wrap other errors
    throw new ApiError(
      error instanceof Error ? error.message : "An unexpected error occurred",
      "unknown"
    );
  }
}

/**
 * Direct backend client (for server-side use with access token)
 */
export async function directBackendClient<T>(
  endpoint: string,
  accessToken: string,
  options: ApiClientOptions = {}
): Promise<T> {
  const { method = "GET", body, headers = {} } = options;

  const config: RequestInit = {
    method,
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${accessToken}`,
      ...headers,
    },
  };

  if (body) {
    config.body = JSON.stringify(body);
  }

  try {
    const response = await fetch(`${BACKEND_URL}${endpoint}`, config);

    if (!response.ok) {
      const errorText = await response.text().catch(() => "");
      let errorData: any;
      try {
        errorData = errorText ? JSON.parse(errorText) : { error: response.statusText };
      } catch {
        errorData = { error: errorText || response.statusText };
      }
      throw parseErrorResponse(response, errorData);
    }

    if (response.status === 204) {
      return undefined as unknown as T;
    }

    const text = await response.text().catch(() => "");
    if (!text) {
      return undefined as unknown as T;
    }
    return JSON.parse(text) as T;
  } catch (error) {
    // Handle network errors
    if (error instanceof TypeError && error.message.includes("fetch")) {
      throw new ApiError("Network error. Please check your connection.", "network");
    }
    // Re-throw ApiError as-is
    if (error instanceof ApiError) {
      throw error;
    }
    // Wrap other errors
    throw new ApiError(
      error instanceof Error ? error.message : "An unexpected error occurred",
      "unknown"
    );
  }
}
