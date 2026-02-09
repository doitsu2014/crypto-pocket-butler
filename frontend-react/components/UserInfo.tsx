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
        <div className="h-4 bg-slate-700 rounded w-3/4 mb-2"></div>
        <div className="h-4 bg-slate-700 rounded w-1/2"></div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-900/20 border border-red-500/50 rounded-lg p-4">
        <p className="text-red-300 text-sm">
          <strong>Error:</strong> {error}
        </p>
      </div>
    );
  }

  if (!userInfo) {
    return null;
  }

  return (
    <div className="bg-violet-900/20 border border-violet-500/30 rounded-lg p-4 backdrop-blur-sm">
      <h4 className="text-sm font-semibold text-violet-300 mb-2 flex items-center gap-2">
        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
        </svg>
        User Information from Backend
      </h4>
      <dl className="space-y-1 text-sm">
        <div>
          <dt className="inline font-medium text-slate-300">User ID: </dt>
          <dd className="inline text-slate-400">{userInfo.user_id}</dd>
        </div>
        {userInfo.preferred_username && (
          <div>
            <dt className="inline font-medium text-slate-300">Username: </dt>
            <dd className="inline text-slate-400">{userInfo.preferred_username}</dd>
          </div>
        )}
        {userInfo.email && (
          <div>
            <dt className="inline font-medium text-slate-300">Email: </dt>
            <dd className="inline text-slate-400">{userInfo.email}</dd>
          </div>
        )}
        {userInfo.email_verified !== undefined && (
          <div>
            <dt className="inline font-medium text-slate-300">Email Verified: </dt>
            <dd className="inline text-slate-400">
              <span className={`inline-flex items-center gap-1 ${userInfo.email_verified ? 'text-green-400' : 'text-yellow-400'}`}>
                {userInfo.email_verified ? (
                  <>
                    <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                    Yes
                  </>
                ) : (
                  <>
                    <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                    </svg>
                    No
                  </>
                )}
              </span>
            </dd>
          </div>
        )}
      </dl>
    </div>
  );
}
