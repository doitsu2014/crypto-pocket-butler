# External Dependency Map — Users, Roles & Permissions

Gated items outside this team, mapped to the Bolts that consume them. A **Bolt** is one build pass over a Unit of Work, ending in something that runs.

| # | Dependency | Owner | What it provides | Lead time | Blocks | If it slips |
|---|---|---|---|---|---|---|
| X1 | Supabase Auth Admin API | Platform / Supabase configuration | Ability to write `app_metadata.role` and audit the previous and new value | Unknown — must be proven in B4 | B4 (`u2-authorisation`), B10 (`u10-admin-users-ui`) | Implement the decision path without the write; B4 does not close until a role change can be performed and audited |
| X2 | IP geolocation provider | Platform / procurement | Continued-use and sign-up location signal for the jurisdiction gate | Unknown — provider not yet chosen | B1 (`u6-jurisdiction-gate`), B3 (`u8-signup-gates-ui`) | Ship on the self-declared country alone with the stricter-signal rule still stated; the IP signal is added when the provider is available |
| X3 | Account and Portfolio service APIs | Account and Portfolio units (existing services) | Account and portfolio identifiers, labels, source type, sync status, timestamps for the support metadata projection | Existing services; contract confirmation needed | B7 (`u3-support-access`), B9 (`u9-support-console-ui`) | Support shows identifiers and sync state from what the existing APIs expose; no cross-schema read is introduced |
| X4 | Restricted-jurisdiction list | Compliance | The launch list of restricted jurisdictions and its review cadence | Unknown | B1 (`u6-jurisdiction-gate`) | B1 ships with an empty list and a loud startup warning; the gate is wired but inert until the list is populated |

## Notes

- Nothing else in this plan depends on a party outside the team: the UI packages, the Identity service, the gateway route table, and the migration pairs are all inside the repository.
- X2 and X4 are the two that most directly gate the walking skeleton; both are recorded in the risk register.
- A dependency that slips does not silently disappear: the Bolt that consumes it either closes with a stated fallback (as above) or stays open.
