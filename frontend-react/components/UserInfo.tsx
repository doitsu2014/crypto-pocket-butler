"use client";

import { useEffect, useState } from "react";

interface BackendUserInfo {
  user_id: string;
  preferred_username?: string;
  email?: string;
  email_verified?: boolean;
}

export function UserInfo() {
  const [userInfo, setUserInfo] = useState<BackendUserInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function fetchUserInfo() {
      try {
        const response = await fetch("/api/backend/me");
        
        if (!response.ok) {
          throw new Error(`Failed to fetch user info: ${response.statusText}`);
        }
        
        const data = await response.json();
        setUserInfo(data);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Unknown error");
      } finally {
        setLoading(false);
      }
    }

    fetchUserInfo();
  }, []);

  if (loading) {
    return (
      <div className="animate-pulse">
        <div className="h-4 bg-gray-200 rounded w-3/4 mb-2"></div>
        <div className="h-4 bg-gray-200 rounded w-1/2"></div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-50 border border-red-200 rounded-md p-4">
        <p className="text-red-800 text-sm">
          <strong>Error:</strong> {error}
        </p>
      </div>
    );
  }

  if (!userInfo) {
    return null;
  }

  return (
    <div className="bg-blue-50 border border-blue-200 rounded-md p-4">
      <h4 className="text-sm font-semibold text-gray-900 mb-2">
        User Information from Backend
      </h4>
      <dl className="space-y-1 text-sm">
        <div>
          <dt className="inline font-medium text-gray-700">User ID: </dt>
          <dd className="inline text-gray-600">{userInfo.user_id}</dd>
        </div>
        {userInfo.preferred_username && (
          <div>
            <dt className="inline font-medium text-gray-700">Username: </dt>
            <dd className="inline text-gray-600">{userInfo.preferred_username}</dd>
          </div>
        )}
        {userInfo.email && (
          <div>
            <dt className="inline font-medium text-gray-700">Email: </dt>
            <dd className="inline text-gray-600">{userInfo.email}</dd>
          </div>
        )}
        {userInfo.email_verified !== undefined && (
          <div>
            <dt className="inline font-medium text-gray-700">Email Verified: </dt>
            <dd className="inline text-gray-600">
              {userInfo.email_verified ? "Yes" : "No"}
            </dd>
          </div>
        )}
      </dl>
    </div>
  );
}
