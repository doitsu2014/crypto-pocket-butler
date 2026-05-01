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
        <div className="h-4 bg-violet-900/50 rounded w-3/4 mb-2 shadow-[0_0_10px_rgba(139,92,246,0.3)]"></div>
        <div className="h-4 bg-violet-900/50 rounded w-1/2 shadow-[0_0_10px_rgba(139,92,246,0.3)]"></div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-950/30 border-2 border-red-500/50 rounded-xl p-4 shadow-[0_0_30px_rgba(239,68,68,0.3)]">
        <p className="text-red-300 text-sm drop-shadow-[0_0_5px_rgba(252,165,165,0.5)]">
          <strong>Error:</strong> {error}
        </p>
      </div>
    );
  }

  if (!userInfo) {
    return null;
  }

  return (
    <div className="bg-fuchsia-950/20 border-2 border-fuchsia-500/40 rounded-xl p-4 backdrop-blur-sm shadow-[0_0_30px_rgba(217,70,239,0.3)]">
      <h4 className="text-sm font-bold text-fuchsia-300 mb-2 flex items-center gap-2 drop-shadow-[0_0_10px_rgba(232,121,249,0.6)]">
        <svg className="w-5 h-5 drop-shadow-[0_0_10px_rgba(232,121,249,0.8)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
        </svg>
        User Information from Backend
      </h4>
      <dl className="space-y-1 text-sm">
        <div>
          <dt className="inline font-bold text-slate-200">User ID: </dt>
          <dd className="inline text-slate-300">{userInfo.user_id}</dd>
        </div>
        {userInfo.preferred_username && (
          <div>
            <dt className="inline font-bold text-slate-200">Username: </dt>
            <dd className="inline text-slate-300">{userInfo.preferred_username}</dd>
          </div>
        )}
        {userInfo.email && (
          <div>
            <dt className="inline font-bold text-slate-200">Email: </dt>
            <dd className="inline text-slate-300">{userInfo.email}</dd>
          </div>
        )}
        {userInfo.email_verified !== undefined && (
          <div>
            <dt className="inline font-bold text-slate-200">Email Verified: </dt>
            <dd className="inline text-slate-300">
              <span className={`inline-flex items-center gap-1 ${userInfo.email_verified ? 'text-green-400 drop-shadow-[0_0_10px_rgba(74,222,128,0.6)]' : 'text-yellow-400 drop-shadow-[0_0_10px_rgba(250,204,21,0.6)]'}`}>
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
