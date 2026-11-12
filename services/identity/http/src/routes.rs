//! The Identity router. Every authenticated path names its identity: the
//! forwarded Supabase JWT for the user surface, and a per-caller service token
//! for each internal path. A path with no rule is not served.
//!
//! The four support-access paths (FR16.3–FR16.6, BR3.10) sit on the user
//! surface: request, approve, revoke and read the metadata of a live grant.

use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{delete, get, patch, post, put};
use axum::Router;

use crate::authorisation::{
    authorize, change_role, change_role_for_gateway, require_internal_caller,
};
use crate::consent::{
    consent_history, grant_consent, internal_consent_history, internal_consent_state,
    internal_grant_or_withdraw, internal_read_preferences, internal_write_preferences,
    my_consent_state, read_preferences, retired_consent_append, withdraw_consent,
    write_preferences,
};
use crate::handlers::{
    approve_support_grant, cancel_deletion, cancel_my_deletion_request, check_jurisdiction,
    disable_account, fallback, get_consent, get_contact, get_deletion, get_due_users, get_export,
    get_user, health, internal_get_by_subject, internal_get_user, internal_lookup,
    internal_provision, list_pending_grants, list_sessions, lookup_admin_user, metrics,
    my_account_status, my_deletion_request, my_profile, my_support_grants, publish_jurisdiction,
    ready, request_deletion, request_export, request_support_grant, resolve_principal,
    restore_account, revoke_session, revoke_support_grant, run_privacy_sweep, signup_gate,
    support_grant_metadata, update_my_profile, update_preferences,
};
use crate::middleware::{
    correlation_id, record_metrics, require_contact, require_due_users, require_jurisdiction_check,
    require_jwt, require_registry_read, require_registry_write, require_run, security_headers,
};
use crate::state::AppState;

/// Builds the router.
pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/metrics", get(metrics))
        .route("/v1/signup-gate", post(signup_gate));

    let authenticated = Router::new()
        .route("/v1/principals/{authUserId}", get(resolve_principal))
        .route("/v1/consent", post(retired_consent_append))
        .route("/v1/consent/{userId}", get(get_consent))
        .route("/v1/users/{userId}/preferences", patch(update_preferences))
        .route("/v1/users/{userId}/sessions", get(list_sessions))
        .route(
            "/v1/users/{userId}/sessions/{sessionId}",
            delete(revoke_session),
        )
        .route("/v1/exports", post(request_export))
        .route("/v1/exports/{exportId}", get(get_export))
        .route("/v1/privacy/deletions", post(request_deletion))
        .route("/v1/privacy/deletions/{deletionId}", get(get_deletion))
        .route(
            "/v1/support-grants",
            get(list_pending_grants).post(request_support_grant),
        )
        .route("/v1/support-grants/mine", get(my_support_grants))
        .route(
            "/v1/support-grants/{grantId}/approve",
            post(approve_support_grant),
        )
        .route("/v1/support-grants/{grantId}", delete(revoke_support_grant))
        .route(
            "/v1/support-grants/{grantId}/metadata",
            get(support_grant_metadata),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), require_jwt));

    // U1: the user record read (owner or admin) and the U10 admin exact-email
    // lookup. Both authenticate with the forwarded JWT; the admin role comes
    // from that token. D-S1: the lookup address travels in the body.
    let user_registry = Router::new()
        .route("/v1/users/{userId}", get(get_user))
        .route("/v1/admin/users/lookup", post(lookup_admin_user))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_jwt));

    // U2/U10: the gateway-facing admin surface. The gateway requires the admin
    // role at the edge and Identity re-checks it from the same forwarded JWT.
    // The status paths reuse the C7 handlers; their internal service-token
    // routes stay in place above.
    let admin_surface = Router::new()
        .route(
            "/v1/admin/users/{userId}/role",
            put(change_role_for_gateway),
        )
        .route("/v1/admin/users/{userId}/disable", post(disable_account))
        .route("/v1/admin/users/{userId}/restore", post(restore_account))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_jwt));

    // C3: the internal registry reads. The gateway reads for the admin surface;
    // the release smoke check resolves one principal by subject hash.
    let registry_read = Router::new()
        .route(
            "/v1/internal/users/by-subject/{subjectHash}",
            get(internal_get_by_subject),
        )
        .route("/v1/internal/users/{userId}", get(internal_get_user))
        .route("/v1/internal/users", get(internal_lookup))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_registry_read,
        ));

    // C3: only the gateway may provision or refresh a record.
    let registry_write = Router::new()
        .route(
            "/v1/internal/users/by-subject/{subjectHash}",
            put(internal_provision),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_registry_write,
        ));

    // Each internal path names the one service that may call it.
    let contact = Router::new()
        .route("/v1/users/{userId}/contact", get(get_contact))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_contact,
        ));

    let due = Router::new()
        .route("/v1/internal/users/due", get(get_due_users))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_due_users,
        ));

    let runs = Router::new()
        .route(
            "/v1/internal/run/audit-privacy-sweep",
            post(run_privacy_sweep),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), require_run));

    // C8: the jurisdiction gate. The check names its trusted callers; publish is
    // an administrator action validated from the forwarded JWT.
    let jurisdiction_check = Router::new()
        .route("/v1/internal/jurisdiction/check", post(check_jurisdiction))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_jurisdiction_check,
        ));

    let jurisdiction_publish = Router::new()
        .route(
            "/v1/internal/jurisdiction/publish",
            post(publish_jurisdiction),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), require_jwt));

    // U2/C4: the authorisation decision and the admin role change. Both take
    // the calling service's token and the acting user's JWT; the paths are
    // reached directly on the internal network, never through the gateway's
    // /v1 table.
    let internal_authorisation = Router::new()
        .route("/internal/authorize", post(authorize))
        .route("/internal/admin/role", put(change_role))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_internal_caller,
        ));

    // U4: the person's own preferences and consent surface. The user id comes
    // from the validated JWT, never from a path.
    let me = Router::new()
        .route(
            "/v1/me/preferences",
            get(read_preferences).put(write_preferences),
        )
        .route("/v1/me/consent/state", get(my_consent_state))
        .route("/v1/me/consent", get(consent_history).post(grant_consent))
        .route("/v1/me/consent/{kind}/withdraw", post(withdraw_consent))
        // U7: the settings surface's self-scoped reads and writes. D0, D1 and
        // D2 resolve the caller by auth subject so a denied account can still
        // read its own status and restore itself.
        .route("/v1/me/account-status", get(my_account_status))
        .route("/v1/me/profile", get(my_profile).patch(update_my_profile))
        .route(
            "/v1/me/deletion-request",
            get(my_deletion_request).delete(cancel_my_deletion_request),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), require_jwt));

    // C6: the source-connection path asks for the token's own user. The
    // allowlisted service token and the forwarded JWT are both required. The
    // declared contract routes are the preferences pair, the history/grant pair,
    // and the consent-state read.
    let internal_consent = Router::new()
        .route(
            "/internal/users/{userId}/preferences",
            get(internal_read_preferences).put(internal_write_preferences),
        )
        .route(
            "/internal/users/{userId}/consent",
            get(internal_consent_history).post(internal_grant_or_withdraw),
        )
        .route(
            "/internal/users/{userId}/consent/state",
            get(internal_consent_state),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_internal_caller,
        ));

    // C7: the account lifecycle transitions. Each names its trusted callers with
    // the service-token allowlist and carries the acting user's JWT; the owner
    // paths answer 404 for another id.
    let account_lifecycle = Router::new()
        .route("/internal/users/{userId}/disable", post(disable_account))
        .route("/internal/users/{userId}/restore", post(restore_account))
        .route(
            "/internal/users/{userId}/deletion-requests",
            post(request_deletion).delete(cancel_deletion),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_internal_caller,
        ));

    Router::new()
        .merge(public)
        .merge(authenticated)
        .merge(me)
        .merge(internal_consent)
        .merge(user_registry)
        .merge(admin_surface)
        .merge(registry_read)
        .merge(registry_write)
        .merge(contact)
        .merge(due)
        .merge(runs)
        .merge(jurisdiction_check)
        .merge(jurisdiction_publish)
        .merge(internal_authorisation)
        .merge(account_lifecycle)
        .fallback(fallback)
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(middleware::from_fn(security_headers))
        .layer(middleware::from_fn(correlation_id))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            record_metrics,
        ))
        .with_state(state)
}
