//! Port fakes for gateway use-case and HTTP tests.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use gateway_application::ports::{
    AuditError, AuditEvent, AuditSink, AuthError, Clock, ForwardRequest, IncomingRequest,
    Principal, ReadinessProbe, TokenValidator, UpstreamClient, UpstreamError, UpstreamResponse,
};

pub struct FakeValidator {
    pub principals: Mutex<HashMap<String, Result<Principal, AuthError>>>,
}

impl FakeValidator {
    pub fn with_token(token: &str, principal: Principal) -> Self {
        let mut principals = HashMap::new();
        principals.insert(token.to_string(), Ok(principal));
        Self {
            principals: Mutex::new(principals),
        }
    }

    pub fn user() -> Self {
        Self::with_token(
            "valid-user-token",
            Principal {
                subject: "user-1".to_string(),
                role: gateway_domain::Role::User,
            },
        )
    }

    pub fn admin() -> Self {
        Self::with_token(
            "valid-admin-token",
            Principal {
                subject: "admin-1".to_string(),
                role: gateway_domain::Role::Admin,
            },
        )
    }

    pub fn unavailable() -> Self {
        let mut principals = HashMap::new();
        principals.insert(
            "valid-user-token".to_string(),
            Err(AuthError::Unavailable("jwks down".to_string())),
        );
        Self {
            principals: Mutex::new(principals),
        }
    }
}

#[async_trait]
impl TokenValidator for FakeValidator {
    async fn validate(&self, token: &str) -> Result<Principal, AuthError> {
        match self.principals.lock().unwrap().get(token) {
            Some(result) => result.clone(),
            None => Err(AuthError::Invalid("unknown test token".to_string())),
        }
    }
}

pub struct FakeUpstream {
    pub responses: Mutex<VecDeque<Result<UpstreamResponse, UpstreamError>>>,
    pub requests: Mutex<Vec<ForwardRequest>>,
}

impl FakeUpstream {
    pub fn with_responses(responses: Vec<Result<UpstreamResponse, UpstreamError>>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            requests: Mutex::new(Vec::new()),
        }
    }

    pub fn ok() -> Self {
        Self::with_responses(vec![Ok(UpstreamResponse {
            status: 200,
            headers: Vec::new(),
            body: br#"{"ok":true}"#.to_vec(),
            content_type: Some("application/json".to_string()),
        })])
    }

    pub fn calls(&self) -> usize {
        self.requests.lock().unwrap().len()
    }

    pub fn last_request(&self) -> ForwardRequest {
        self.requests
            .lock()
            .unwrap()
            .last()
            .cloned()
            .expect("at least one forwarded request")
    }
}

#[async_trait]
impl UpstreamClient for FakeUpstream {
    async fn forward(&self, request: ForwardRequest) -> Result<UpstreamResponse, UpstreamError> {
        self.requests.lock().unwrap().push(request);
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| {
                Ok(UpstreamResponse {
                    status: 200,
                    headers: Vec::new(),
                    body: Vec::new(),
                    content_type: None,
                })
            })
    }
}

pub struct FakeAudit {
    pub events: Mutex<Vec<AuditEvent>>,
    pub fail: bool,
}

impl Default for FakeAudit {
    fn default() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            fail: false,
        }
    }
}

impl FakeAudit {
    pub fn events(&self) -> Vec<AuditEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl AuditSink for FakeAudit {
    async fn record(&self, event: &AuditEvent) -> Result<(), AuditError> {
        if self.fail {
            return Err(AuditError::Unavailable("audit down".to_string()));
        }
        self.events.lock().unwrap().push(event.clone());
        Ok(())
    }
}

#[derive(Clone)]
pub struct FixedClock {
    pub now: DateTime<Utc>,
}

impl FixedClock {
    pub fn at(now: DateTime<Utc>) -> Self {
        Self { now }
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.now
    }
}

pub struct Ready(pub bool);

#[async_trait]
impl ReadinessProbe for Ready {
    async fn check(&self) -> Result<(), String> {
        if self.0 {
            Ok(())
        } else {
            Err("upstream down".to_string())
        }
    }
}

pub fn incoming(
    method: gateway_domain::Method,
    path: &str,
    token: Option<&str>,
    correlation: Option<&str>,
) -> IncomingRequest {
    let mut headers = Vec::new();
    if let Some(token) = token {
        headers.push(("authorization".to_string(), format!("Bearer {token}")));
    }
    if let Some(correlation) = correlation {
        headers.push(("x-correlation-id".to_string(), correlation.to_string()));
    }
    IncomingRequest {
        method,
        path: path.to_string(),
        query: None,
        headers,
        body: Vec::new(),
    }
}
