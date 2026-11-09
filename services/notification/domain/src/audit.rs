//! The C10 action names Notification emits.
//!
//! C10 accepts a `system` actor only for an action on its allow-list, so the
//! erasure event uses `system.privacy.deletion`. Delivery and preference
//! refusals are user-attributed and may name Notification's own actions.

/// A mandatory security or revoked-key email reached the provider.
pub const SECURITY_ALERT_DELIVERED: &str = "notification.security_alert.delivered";
/// A mandatory security or revoked-key email was abandoned.
pub const SECURITY_ALERT_ABANDONED: &str = "notification.security_alert.abandoned";
/// A user tried to disable a mandatory email (BR2.1, BR3.1).
pub const PREFERENCE_REFUSED: &str = "notification.preference.refused";
/// Identity's erasure reached Notification (NFR13.1). Allow-listed because it
/// is a system-attributed action.
pub const PRIVACY_ANONYMISED: &str = "system.privacy.deletion";
