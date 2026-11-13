//! sqlx-backed `user_preferences`: one row per user holding only the explicit
//! choices, plus the provider-profile lookup the effective read resolves against.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_application::ports::RepositoryError;
use identity_application::ports::UserRepository;
use identity_application::preferences::ports::{PreferencesRepository, UserLookup};
use identity_domain::consent::parse_iana_timezone;
use identity_domain::{
    BaseCurrencyCode, Locale, PreferenceUpdate, ProviderProfile, StoredPreferences, TimeZone,
    UserId,
};
use sqlx::PgPool;

use crate::repositories::{invalid_data, map_sqlx_error, PgUserRepository};

/// sqlx-backed preferences.
#[derive(Debug, Clone)]
pub struct PgPreferencesRepository {
    pool: PgPool,
}

impl PgPreferencesRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct PreferencesRow {
    locale: Option<String>,
    base_currency: Option<String>,
    timezone: Option<String>,
    notification_email: Option<bool>,
    notification_push: Option<bool>,
}

impl PreferencesRow {
    fn into_domain(self) -> Result<StoredPreferences, RepositoryError> {
        let locale = self
            .locale
            .map(|raw| Locale::parse(&raw))
            .transpose()
            .map_err(|error| invalid_data("read preferences", error))?;
        let base_currency = self
            .base_currency
            .map(|raw| BaseCurrencyCode::parse(&raw))
            .transpose()
            .map_err(|error| invalid_data("read preferences", error))?;
        let timezone = self
            .timezone
            .map(|raw| parse_iana_timezone(&raw))
            .transpose()
            .map_err(|error| invalid_data("read preferences", error))?;
        Ok(StoredPreferences {
            locale,
            base_currency,
            timezone,
            notification_email: self.notification_email,
            notification_push: self.notification_push,
        })
    }
}

#[async_trait]
impl PreferencesRepository for PgPreferencesRepository {
    async fn load(&self, user_id: UserId) -> Result<Option<StoredPreferences>, RepositoryError> {
        let row = sqlx::query_as::<_, PreferencesRow>(
            "SELECT locale, base_currency, timezone, notification_email, notification_push
             FROM user_preferences WHERE user_id = $1",
        )
        .bind(user_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read preferences", error))?;
        row.map(PreferencesRow::into_domain).transpose()
    }

    async fn upsert(
        &self,
        user_id: UserId,
        update: &PreferenceUpdate,
        now: DateTime<Utc>,
    ) -> Result<StoredPreferences, RepositoryError> {
        // `COALESCE(EXCLUDED.x, existing.x)` is the partial write: an unsupplied
        // (NULL) field keeps its stored value, an explicit one replaces it.
        let row = sqlx::query_as::<_, PreferencesRow>(
            "INSERT INTO user_preferences
                 (user_id, locale, base_currency, timezone, notification_email,
                  notification_push, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (user_id) DO UPDATE SET
                 locale = COALESCE(EXCLUDED.locale, user_preferences.locale),
                 base_currency = COALESCE(EXCLUDED.base_currency, user_preferences.base_currency),
                 timezone = COALESCE(EXCLUDED.timezone, user_preferences.timezone),
                 notification_email =
                     COALESCE(EXCLUDED.notification_email, user_preferences.notification_email),
                 notification_push =
                     COALESCE(EXCLUDED.notification_push, user_preferences.notification_push),
                 updated_at = EXCLUDED.updated_at
             RETURNING locale, base_currency, timezone, notification_email, notification_push",
        )
        .bind(user_id.as_uuid())
        .bind(update.locale.map(Locale::as_str))
        .bind(update.base_currency.map(BaseCurrencyCode::as_str))
        .bind(update.timezone.as_ref().map(TimeZone::as_str))
        .bind(update.notification_email)
        .bind(update.notification_push)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("write preferences", error))?;
        row.into_domain()
    }
}

/// The provider-profile lookup reuses u1's stored user row; U4 only reads it.
#[async_trait]
impl UserLookup for PgUserRepository {
    async fn provider_profile(
        &self,
        user_id: UserId,
    ) -> Result<Option<ProviderProfile>, RepositoryError> {
        let user = self.find_by_id(user_id).await?;
        Ok(user
            .map(|user| ProviderProfile::new(user.locale(), user.base_currency(), user.timezone())))
    }
}
