//! Application tests: portfolio creation and membership, backend valuation with
//! provenance and degraded states, targets and drift, snapshots, intents,
//! recorded trades and overlays, guardrail crossings, the scheduled run, privacy,
//! and the service allow-list — all against port fakes.

mod fakes;

use std::str::FromStr;
use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use portfolio_application::common::{AppError, AssuranceLevel, Caller, ServiceCaller};
use portfolio_application::guardrails::GuardrailService;
use portfolio_application::intents::{IntentRequest, IntentService};
use portfolio_application::portfolios::PortfolioService;
use portfolio_application::ports::{
    GuardrailRepository, HoldingObservation, MembershipRepository, RunRepository, RunStatus,
    TargetRepository, ValuationRun, VenueTradeObservation,
};
use portfolio_application::privacy::PrivacyService;
use portfolio_application::run::ValuationRunService;
use portfolio_application::snapshot::{freeze_inputs, lines_json, verify_snapshot, FrozenInputs};
use portfolio_application::targets::{TargetInput, TargetService};
use portfolio_application::trades::{TradeRequest, TradeService};
use portfolio_application::trail::Trail;
use portfolio_application::valuation::ValuationService;
use portfolio_domain::{
    AccountId, Freshness, FxRate, Guardrail, GuardrailId, OrderSide, Portfolio, PortfolioAccount,
    PortfolioAccountId, PortfolioId, PriceSource, Quote, TargetAllocation, TargetAllocationId,
    UserId,
};
use rust_decimal::Decimal;
use serde_json::json;

use fakes::{
    FakeAudit, FakeHoldings, FakeIntents, FakeLedger, FakeMarket, FakeMemberships,
    FakeNotifications, FakeOverlays, FakePortfolios, FakePreferences, FakePrivacy, FakeRuns,
    FakeSnapshots, FakeTargets, FakeTrades, MutableClock, Owners, SeqIds,
};

const USER_A: &str = "11111111-1111-4111-8111-111111111111";
const USER_B: &str = "22222222-2222-4222-8222-222222222222";
const ACCOUNT: &str = "33333333-3333-4333-8333-333333333333";

fn d(raw: &str) -> Decimal {
    Decimal::from_str(raw).unwrap()
}

fn user_a() -> UserId {
    UserId::parse(USER_A).unwrap()
}

fn user_b() -> UserId {
    UserId::parse(USER_B).unwrap()
}

fn account() -> AccountId {
    AccountId::parse(ACCOUNT).unwrap()
}

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, hour, 0, 0).unwrap()
}

fn caller() -> Caller {
    Caller {
        user_id: user_a(),
        assurance: AssuranceLevel::Aal2,
        correlation_id: "corr-test".to_string(),
    }
}

fn other_caller() -> Caller {
    Caller {
        user_id: user_b(),
        assurance: AssuranceLevel::Aal2,
        correlation_id: "corr-other".to_string(),
    }
}

fn service(name: &str) -> ServiceCaller {
    ServiceCaller::new(name, "corr-service")
}

struct Harness {
    clock: MutableClock,
    portfolios_repo: Arc<FakePortfolios>,
    memberships: Arc<FakeMemberships>,
    targets: Arc<FakeTargets>,
    snapshots: Arc<FakeSnapshots>,
    overlays: Arc<FakeOverlays>,
    intents_repo: Arc<FakeIntents>,
    trades_repo: Arc<FakeTrades>,
    runs: Arc<FakeRuns>,
    holdings: Arc<FakeHoldings>,
    market: Arc<FakeMarket>,
    ledger: Arc<FakeLedger>,
    notifications: Arc<FakeNotifications>,
    audit: Arc<FakeAudit>,
    portfolios: PortfolioService,
    valuation: Arc<ValuationService>,
    targets_service: TargetService,
    intents: IntentService,
    trades: TradeService,
    guardrails: Arc<GuardrailService>,
    run: ValuationRunService,
    privacy: PrivacyService,
}

fn harness(now: DateTime<Utc>, preference: Option<&str>) -> Harness {
    let clock = MutableClock::new(now);
    let owners: Owners = Arc::default();
    let portfolios_repo = Arc::new(FakePortfolios::new(owners.clone()));
    let memberships = Arc::new(FakeMemberships::new(owners.clone()));
    let targets = Arc::new(FakeTargets::default());
    let snapshots = Arc::new(FakeSnapshots::new(owners.clone()));
    let overlays = Arc::new(FakeOverlays::new(owners.clone()));
    let intents_repo = Arc::new(FakeIntents::new(owners.clone()));
    let trades_repo = Arc::new(FakeTrades::new(owners.clone(), overlays.clone()));
    let runs = Arc::new(FakeRuns::default());
    let privacy_repo = Arc::new(FakePrivacy::default());
    let holdings = Arc::new(FakeHoldings::default());
    let market = Arc::new(FakeMarket::default());
    let ledger = Arc::new(FakeLedger::default());
    let notifications = Arc::new(FakeNotifications::default());
    let audit = Arc::new(FakeAudit::default());
    let preferences = Arc::new(FakePreferences::new(preference));
    let ids: Arc<SeqIds> = Arc::new(SeqIds::default());

    let trail = Arc::new(Trail::new(audit.clone(), Arc::new(clock.clone())));
    let valuation = Arc::new(ValuationService::new(
        portfolios_repo.clone(),
        memberships.clone(),
        targets.clone(),
        snapshots.clone(),
        overlays.clone(),
        holdings.clone(),
        market.clone(),
        Arc::new(clock.clone()),
        ids.clone(),
    ));
    let portfolios = PortfolioService::new(
        portfolios_repo.clone(),
        memberships.clone(),
        preferences,
        Arc::new(clock.clone()),
        ids.clone(),
    );
    let targets_service = TargetService::new(
        portfolios_repo.clone(),
        targets.clone(),
        trail.clone(),
        Arc::new(clock.clone()),
        ids.clone(),
    );
    let intents = IntentService::new(
        portfolios_repo.clone(),
        intents_repo.clone(),
        market.clone(),
        valuation.clone(),
        trail.clone(),
        Arc::new(clock.clone()),
        ids.clone(),
    );
    let trades = TradeService::new(
        portfolios_repo.clone(),
        trades_repo.clone(),
        ledger.clone(),
        trail.clone(),
        Arc::new(clock.clone()),
        ids.clone(),
    );
    let guardrail_repo: Arc<dyn GuardrailRepository> = targets.clone();
    let target_repo: Arc<dyn TargetRepository> = targets.clone();
    let guardrails = Arc::new(GuardrailService::new(
        target_repo,
        guardrail_repo,
        notifications.clone(),
        Arc::new(clock.clone()),
    ));
    let run = ValuationRunService::new(
        portfolios_repo.clone(),
        memberships.clone(),
        overlays.clone(),
        holdings.clone(),
        trades_repo.clone(),
        runs.clone(),
        valuation.clone(),
        guardrails.clone(),
        Arc::new(clock.clone()),
        ids.clone(),
    );
    let privacy = PrivacyService::new(
        portfolios_repo.clone(),
        memberships.clone(),
        targets.clone(),
        snapshots.clone(),
        overlays.clone(),
        intents_repo.clone(),
        trades_repo.clone(),
        privacy_repo,
        trail,
        Arc::new(clock.clone()),
        ids.clone(),
    );

    Harness {
        clock,
        portfolios_repo,
        memberships,
        targets,
        snapshots,
        overlays,
        intents_repo,
        trades_repo,
        runs,
        holdings,
        market,
        ledger,
        notifications,
        audit,
        portfolios,
        valuation,
        targets_service,
        intents,
        trades,
        guardrails,
        run,
        privacy,
    }
}

impl Harness {
    fn seed_portfolio(&self, user: UserId) -> Portfolio {
        self.seed_portfolio_with_account(user, account())
    }

    fn seed_portfolio_with_account(&self, user: UserId, account_id: AccountId) -> Portfolio {
        let name = format!("Portfolio {}", PortfolioId::generate());
        let portfolio = Portfolio::new(PortfolioId::generate(), user, &name, "USD", at(0)).unwrap();
        self.portfolios_repo.seed(portfolio.clone());
        self.memberships.seed(PortfolioAccount::new(
            PortfolioAccountId::generate(),
            portfolio.id(),
            account_id,
            at(0),
        ));
        portfolio
    }

    fn seed_holding(&self, asset: &str, quantity: &str) {
        self.seed_holding_for(account(), asset, quantity);
    }

    fn seed_holding_for(&self, account_id: AccountId, asset: &str, quantity: &str) {
        self.holdings.seed(HoldingObservation {
            account_id,
            asset_id: asset.to_string(),
            quantity: d(quantity),
            observed_at: at(0),
        });
    }

    fn seed_quote(&self, asset: &str, price: &str, currency: &str) {
        self.market.seed_quote(
            Quote::new(
                asset,
                Some(d(price)),
                Some(currency.to_string()),
                Some("test".to_string()),
                Some(at(0)),
                Freshness::Live,
            )
            .unwrap(),
        );
    }

    fn seed_target(&self, portfolio: &Portfolio, asset: &str, percent: &str) {
        self.targets.seed_target(
            TargetAllocation::new(
                TargetAllocationId::generate(),
                portfolio.id(),
                asset,
                d(percent),
                at(0),
            )
            .unwrap(),
        );
    }

    fn seed_guardrail(&self, portfolio: &Portfolio, asset: &str, threshold: &str) {
        self.targets.seed_guardrail(
            Guardrail::new(
                GuardrailId::generate(),
                portfolio.id(),
                asset,
                d(threshold),
                at(0),
            )
            .unwrap(),
        );
    }
}

#[tokio::test]
async fn list_or_create_creates_the_default_portfolio() {
    let harness = harness(at(0), Some("EUR"));
    let portfolios = harness.portfolios.list_or_create(&caller()).await.unwrap();
    assert_eq!(portfolios.len(), 1);
    assert_eq!(portfolios[0].base_currency(), "EUR");
    assert_eq!(portfolios[0].name(), "My portfolio");
}

#[tokio::test]
async fn list_or_create_returns_the_existing_portfolio() {
    let harness = harness(at(0), Some("USD"));
    harness.seed_portfolio(user_a());
    let portfolios = harness.portfolios.list_or_create(&caller()).await.unwrap();
    assert_eq!(portfolios.len(), 1);
}

#[tokio::test]
async fn list_or_create_defaults_to_usd_without_a_preference() {
    let harness = harness(at(0), None);
    let portfolios = harness.portfolios.list_or_create(&caller()).await.unwrap();
    assert_eq!(portfolios[0].base_currency(), "USD");
}

#[tokio::test]
async fn create_accepts_an_explicit_name_and_currency() {
    let harness = harness(at(0), None);
    let portfolio = harness
        .portfolios
        .create(&caller(), Some("Retirement"), Some("gbp"))
        .await
        .unwrap();
    assert_eq!(portfolio.name(), "Retirement");
    assert_eq!(portfolio.base_currency(), "GBP");
}

#[tokio::test]
async fn create_refuses_a_duplicate_name() {
    let harness = harness(at(0), None);
    harness
        .portfolios
        .create(&caller(), Some("Retirement"), None)
        .await
        .unwrap();
    let error = harness
        .portfolios
        .create(&caller(), Some("retirement"), None)
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Conflict(_)));
}

#[tokio::test]
async fn owned_refuses_another_users_portfolio() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    assert!(matches!(
        harness
            .portfolios
            .owned(&other_caller(), portfolio.id())
            .await
            .unwrap_err(),
        AppError::Forbidden(_)
    ));
}

#[tokio::test]
async fn assign_account_moves_it_into_the_portfolio() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    let membership = harness
        .portfolios
        .assign_account(&caller(), portfolio.id(), account())
        .await
        .unwrap();
    assert_eq!(membership.account_id(), account());
    assert_eq!(
        harness
            .memberships
            .list_for_portfolio(portfolio.id())
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn target_preview_sums_and_reports_the_difference() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    let preview = harness
        .targets_service
        .preview(
            portfolio.id(),
            &[
                TargetInput {
                    asset_id: "a".to_string(),
                    target_percent: d("40"),
                    guardrail_points: None,
                },
                TargetInput {
                    asset_id: "b".to_string(),
                    target_percent: d("50"),
                    guardrail_points: None,
                },
            ],
        )
        .unwrap();
    assert_eq!(preview.sum, d("90"));
    assert_eq!(preview.difference, d("10"));
}

#[tokio::test]
async fn saving_targets_persists_them_and_audits() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    let saved = harness
        .targets_service
        .save(
            &caller(),
            portfolio.id(),
            &[TargetInput {
                asset_id: "a".to_string(),
                target_percent: d("60"),
                guardrail_points: Some(d("5")),
            }],
        )
        .await
        .unwrap();
    assert_eq!(saved.targets.len(), 1);
    assert_eq!(saved.guardrails.len(), 1);
    assert_eq!(harness.targets.targets().len(), 1);
    assert!(harness
        .audit
        .records()
        .iter()
        .any(|record| record.action() == "portfolio.targets.changed"));
}

#[tokio::test]
async fn saving_an_out_of_range_target_is_refused() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    let error = harness
        .targets_service
        .save(
            &caller(),
            portfolio.id(),
            &[TargetInput {
                asset_id: "a".to_string(),
                target_percent: d("101"),
                guardrail_points: None,
            }],
        )
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));
}

#[tokio::test]
async fn saving_a_duplicate_asset_is_refused() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    let error = harness
        .targets_service
        .save(
            &caller(),
            portfolio.id(),
            &[
                TargetInput {
                    asset_id: "a".to_string(),
                    target_percent: d("10"),
                    guardrail_points: None,
                },
                TargetInput {
                    asset_id: "a".to_string(),
                    target_percent: d("20"),
                    guardrail_points: None,
                },
            ],
        )
        .await
        .unwrap_err();
    assert!(
        matches!(error, AppError::Validation(errors) if errors[0].reason.contains("more than once"))
    );
}

#[tokio::test]
async fn saving_a_guardrail_without_a_target_is_refused() {
    let portfolio = Portfolio::new(
        PortfolioId::generate(),
        user_a(),
        "My portfolio",
        "USD",
        at(0),
    )
    .unwrap();
    let targets = vec!["b".to_string()];
    assert!(portfolio_domain::require_target_for_guardrail("a", &targets).is_err());
    let _ = portfolio;
}

#[tokio::test]
async fn reading_a_valuation_prices_the_holdings() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "2");
    harness.seed_quote("a", "100", "USD");
    let read = harness
        .valuation
        .read(&caller(), portfolio.id())
        .await
        .unwrap();
    assert_eq!(read.valuation.total_value(), Some(d("200")));
    assert_eq!(read.valuation.lines().len(), 1);
    assert!(!read.served_from_snapshot);
}

#[tokio::test]
async fn reading_a_valuation_counts_unpriced_assets() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_holding("b", "3");
    harness.seed_quote("a", "100", "USD");
    let read = harness
        .valuation
        .read(&caller(), portfolio.id())
        .await
        .unwrap();
    assert_eq!(read.valuation.status().as_str(), "partial");
    assert_eq!(read.valuation.unpriced_count(), 1);
    assert_eq!(read.valuation.total_value(), Some(d("100")));
}

#[tokio::test]
async fn reading_a_valuation_degrades_when_c5_is_unavailable() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.holdings.set_unavailable(true);
    let read = harness
        .valuation
        .read(&caller(), portfolio.id())
        .await
        .unwrap();
    assert_eq!(read.valuation.status().as_str(), "partial");
    assert!(read.valuation.holdings_unavailable());
    assert!(read.valuation.lines().is_empty());
}

#[tokio::test]
async fn reading_a_valuation_degrades_when_c6_is_unavailable() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    harness.market.set_unavailable(true);
    let read = harness
        .valuation
        .read(&caller(), portfolio.id())
        .await
        .unwrap();
    assert_eq!(read.valuation.status().as_str(), "partial");
    assert_eq!(read.valuation.unpriced_count(), 1);
}

#[tokio::test]
async fn reading_a_valuation_converts_a_foreign_holding() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "10");
    harness.seed_quote("a", "100", "EUR");
    harness
        .market
        .seed_fx(FxRate::new("EUR", "USD", d("1.1"), at(0), "ecb", Vec::new()).unwrap());
    let read = harness
        .valuation
        .read(&caller(), portfolio.id())
        .await
        .unwrap();
    assert_eq!(read.valuation.total_value(), Some(d("1100")));
    assert_eq!(read.valuation.lines()[0].fx_rate(), Some(d("1.1")));
}

#[tokio::test]
async fn reading_a_valuation_without_targets_returns_the_prompt() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let read = harness
        .valuation
        .read(&caller(), portfolio.id())
        .await
        .unwrap();
    assert!(!read.targets_configured);
    assert!(read.prompt.is_some());
}

#[tokio::test]
async fn reading_a_valuation_with_targets_reports_drift() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "6");
    harness.seed_holding("b", "4");
    harness.seed_quote("a", "100", "USD");
    harness.seed_quote("b", "100", "USD");
    harness.seed_target(&portfolio, "a", "50");
    let read = harness
        .valuation
        .read(&caller(), portfolio.id())
        .await
        .unwrap();
    assert!(read.targets_configured);
    assert_eq!(
        read.valuation.line("a").unwrap().drift_points(),
        Some(d("10"))
    );
}

#[tokio::test]
async fn reading_a_valuation_serves_a_fresh_snapshot() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let fresh = harness.valuation.fresh(&portfolio).await.unwrap();
    harness
        .valuation
        .store_snapshot(&portfolio, &fresh.valuation, &fresh.inputs, at(0))
        .await
        .unwrap();
    let read = harness
        .valuation
        .read(&caller(), portfolio.id())
        .await
        .unwrap();
    assert!(read.served_from_snapshot);
    assert_eq!(read.valuation.total_value(), Some(d("100")));
}

#[tokio::test]
async fn reading_a_valuation_recomputes_when_the_snapshot_is_stale() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let fresh = harness.valuation.fresh(&portfolio).await.unwrap();
    harness
        .valuation
        .store_snapshot(&portfolio, &fresh.valuation, &fresh.inputs, at(0))
        .await
        .unwrap();
    harness.clock.set(at(0) + chrono::Duration::seconds(61));
    let read = harness
        .valuation
        .read(&caller(), portfolio.id())
        .await
        .unwrap();
    assert!(!read.served_from_snapshot);
    assert_eq!(read.valuation.total_value(), Some(d("100")));
}

#[tokio::test]
async fn a_tampered_snapshot_is_flagged_and_recomputed() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let fresh = harness.valuation.fresh(&portfolio).await.unwrap();
    harness
        .valuation
        .store_snapshot(&portfolio, &fresh.valuation, &fresh.inputs, at(0))
        .await
        .unwrap();
    harness.snapshots.tamper(0);
    let read = harness
        .valuation
        .read(&caller(), portfolio.id())
        .await
        .unwrap();
    assert!(read.digest_mismatch);
    assert!(!read.served_from_snapshot);
}

#[tokio::test]
async fn a_second_snapshot_in_the_same_hour_is_skipped() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let fresh = harness.valuation.fresh(&portfolio).await.unwrap();
    let first = harness
        .valuation
        .store_snapshot(&portfolio, &fresh.valuation, &fresh.inputs, at(0))
        .await
        .unwrap();
    let second = harness
        .valuation
        .store_snapshot(
            &portfolio,
            &fresh.valuation,
            &fresh.inputs,
            at(0) + chrono::Duration::minutes(20),
        )
        .await
        .unwrap();
    assert!(first.is_some());
    assert!(second.is_none());
}

#[tokio::test]
async fn history_verifies_each_snapshot() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let fresh = harness.valuation.fresh(&portfolio).await.unwrap();
    harness
        .valuation
        .store_snapshot(&portfolio, &fresh.valuation, &fresh.inputs, at(0))
        .await
        .unwrap();
    let history = harness
        .valuation
        .history(portfolio.id(), None, 100, None)
        .await
        .unwrap();
    assert_eq!(history.snapshots.len(), 1);
    let detail = harness
        .valuation
        .snapshot_detail(portfolio.id(), history.snapshots[0].snapshot.id())
        .await
        .unwrap()
        .unwrap();
    assert!(detail.verified);
}

#[tokio::test]
async fn history_reports_an_unverified_snapshot() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let fresh = harness.valuation.fresh(&portfolio).await.unwrap();
    harness
        .valuation
        .store_snapshot(&portfolio, &fresh.valuation, &fresh.inputs, at(0))
        .await
        .unwrap();
    harness.snapshots.tamper(0);
    // History lists the row without reproducing it; the on-demand detail reports
    // the mismatch.
    let history = harness
        .valuation
        .history(portfolio.id(), None, 100, None)
        .await
        .unwrap();
    assert_eq!(history.snapshots.len(), 1);
    let detail = harness
        .valuation
        .snapshot_detail(portfolio.id(), history.snapshots[0].snapshot.id())
        .await
        .unwrap()
        .unwrap();
    assert!(!detail.verified);
}

#[tokio::test]
async fn frozen_inputs_digest_is_stable_and_verifies() {
    let holding = portfolio_domain::Holding::new(Some(account()), "a", d("1"), at(0)).unwrap();
    let quote = Quote::new(
        "a",
        Some(d("100")),
        Some("USD".to_string()),
        Some("test".to_string()),
        Some(at(0)),
        Freshness::Live,
    )
    .unwrap();
    let valuation = portfolio_domain::compute_valuation(
        "USD",
        std::slice::from_ref(&holding),
        std::slice::from_ref(&quote),
        &[],
        &[],
        true,
    );
    let inputs = freeze_inputs("USD", &[holding], &[quote], &[], &[]);
    let digest = inputs.digest();
    assert_eq!(digest.len(), 64);
    assert_eq!(
        digest,
        FrozenInputs::from_json(&inputs.to_json()).unwrap().digest()
    );
    assert!(verify_snapshot(&inputs.to_json(), &digest, &lines_json(&valuation)).is_ok());
}

#[tokio::test]
async fn a_wrong_digest_fails_verification() {
    let inputs = freeze_inputs("USD", &[], &[], &[], &[]);
    assert!(verify_snapshot(&inputs.to_json(), &"0".repeat(64), "{}").is_err());
}

#[tokio::test]
async fn mismatched_stored_lines_fail_verification() {
    let inputs = freeze_inputs("USD", &[], &[], &[], &[]);
    let digest = inputs.digest();
    let tampered = json!({
        "status": "complete",
        "totalValue": "1",
        "currency": "USD",
        "pricedCoverage": "0",
        "unpricedCount": 0,
        "excludedCount": 0,
        "lines": []
    })
    .to_string();
    assert!(verify_snapshot(&inputs.to_json(), &digest, &tampered).is_err());
}

#[tokio::test]
async fn an_intent_preview_projects_the_allocation() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let projection = harness
        .intents
        .preview(
            &caller(),
            portfolio.id(),
            &IntentRequest {
                asset_id: "a".to_string(),
                side: OrderSide::Buy,
                quantity: d("1"),
            },
        )
        .await
        .unwrap();
    assert_eq!(projection.projected_allocation, d("100"));
    assert!(projection.shortfall.is_none());
}

#[tokio::test]
async fn a_sell_intent_preview_states_the_shortfall() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let projection = harness
        .intents
        .preview(
            &caller(),
            portfolio.id(),
            &IntentRequest {
                asset_id: "a".to_string(),
                side: OrderSide::Sell,
                quantity: d("2"),
            },
        )
        .await
        .unwrap();
    assert_eq!(projection.shortfall, Some(d("1")));
}

#[tokio::test]
async fn recording_an_intent_stores_it_pending() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let intent = harness
        .intents
        .create(
            &caller(),
            portfolio.id(),
            &IntentRequest {
                asset_id: "a".to_string(),
                side: OrderSide::Buy,
                quantity: d("1"),
            },
        )
        .await
        .unwrap();
    assert_eq!(intent.status().as_str(), "pending");
    assert_eq!(harness.intents_repo.all().len(), 1);
}

#[tokio::test]
async fn cancelling_an_intent_keeps_it_visible_and_audits() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let intent = harness
        .intents
        .create(
            &caller(),
            portfolio.id(),
            &IntentRequest {
                asset_id: "a".to_string(),
                side: OrderSide::Buy,
                quantity: d("1"),
            },
        )
        .await
        .unwrap();
    let cancelled = harness
        .intents
        .cancel(&caller(), portfolio.id(), intent.id())
        .await
        .unwrap();
    assert_eq!(cancelled.status().as_str(), "cancelled");
    assert_eq!(harness.intents_repo.all().len(), 1);
    assert!(harness
        .audit
        .records()
        .iter()
        .any(|record| record.action() == "portfolio.intent.cancelled"));
}

#[tokio::test]
async fn cancelling_another_users_intent_is_forbidden() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let intent = harness
        .intents
        .create(
            &caller(),
            portfolio.id(),
            &IntentRequest {
                asset_id: "a".to_string(),
                side: OrderSide::Buy,
                quantity: d("1"),
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        harness
            .intents
            .cancel(&other_caller(), portfolio.id(), intent.id())
            .await
            .unwrap_err(),
        AppError::Forbidden(_)
    ));
}

#[tokio::test]
async fn recording_a_trade_writes_the_ledger_and_an_overlay() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    let trade = harness
        .trades
        .record(&caller(), portfolio.id(), &trade_request("idem-1"))
        .await
        .unwrap();
    assert_eq!(trade.ledger_transaction_id(), Some("ledger-idem-1"));
    assert_eq!(harness.ledger.calls().len(), 1);
    assert_eq!(harness.overlays.all().len(), 1);
}

#[tokio::test]
async fn replaying_a_trade_returns_the_existing_one_without_a_second_ledger_call() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    let first = harness
        .trades
        .record(&caller(), portfolio.id(), &trade_request("idem-1"))
        .await
        .unwrap();
    let second = harness
        .trades
        .record(&caller(), portfolio.id(), &trade_request("idem-1"))
        .await
        .unwrap();
    assert_eq!(first.id(), second.id());
    assert_eq!(harness.ledger.calls().len(), 1);
    assert_eq!(harness.overlays.all().len(), 1);
}

#[tokio::test]
async fn a_trade_without_a_price_is_refused() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    let mut request = trade_request("idem-1");
    request.price = None;
    let error = harness
        .trades
        .record(&caller(), portfolio.id(), &request)
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));
    assert!(harness.ledger.calls().is_empty());
}

#[tokio::test]
async fn a_ledger_failure_refuses_the_trade_and_writes_no_overlay() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.ledger.fail(true);
    let error = harness
        .trades
        .record(&caller(), portfolio.id(), &trade_request("idem-1"))
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Unavailable(_)));
    assert!(harness.overlays.all().is_empty());
    assert!(harness.trades_repo.all().is_empty());
}

#[tokio::test]
async fn a_recorded_trade_changes_the_next_valuation() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    harness
        .trades
        .record(&caller(), portfolio.id(), &trade_request("idem-1"))
        .await
        .unwrap();
    let fresh = harness.valuation.fresh(&portfolio).await.unwrap();
    assert_eq!(fresh.valuation.total_value(), Some(d("200")));
}

#[tokio::test]
async fn a_crossing_notifies_once() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "6");
    harness.seed_holding("b", "4");
    harness.seed_quote("a", "100", "USD");
    harness.seed_quote("b", "100", "USD");
    harness.seed_target(&portfolio, "a", "50");
    harness.seed_guardrail(&portfolio, "a", "5");
    let fresh = harness.valuation.fresh(&portfolio).await.unwrap();
    let report = harness
        .guardrails
        .evaluate(&portfolio, &fresh.valuation)
        .await
        .unwrap();
    assert_eq!(report.crossings, 1);
    assert_eq!(harness.notifications.alerts().len(), 1);
    assert!(harness.targets.guardrails()[0].is_outside());

    let report = harness
        .guardrails
        .evaluate(&portfolio, &fresh.valuation)
        .await
        .unwrap();
    assert_eq!(report.crossings, 0);
    assert_eq!(harness.notifications.alerts().len(), 1);
}

#[tokio::test]
async fn a_failed_alert_leaves_the_state_inside_for_a_retry() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "6");
    harness.seed_holding("b", "4");
    harness.seed_quote("a", "100", "USD");
    harness.seed_quote("b", "100", "USD");
    harness.seed_target(&portfolio, "a", "50");
    harness.seed_guardrail(&portfolio, "a", "5");
    harness.notifications.fail(true);
    let fresh = harness.valuation.fresh(&portfolio).await.unwrap();
    let report = harness
        .guardrails
        .evaluate(&portfolio, &fresh.valuation)
        .await
        .unwrap();
    assert_eq!(report.crossings, 0);
    assert_eq!(report.alerts_failed, 1);
    assert!(!harness.targets.guardrails()[0].is_outside());
}

#[tokio::test]
async fn returning_inside_clears_the_crossing_state() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "6");
    harness.seed_holding("b", "4");
    harness.seed_quote("a", "100", "USD");
    harness.seed_quote("b", "100", "USD");
    harness.seed_target(&portfolio, "a", "50");
    harness.seed_guardrail(&portfolio, "a", "5");
    let fresh = harness.valuation.fresh(&portfolio).await.unwrap();
    harness
        .guardrails
        .evaluate(&portfolio, &fresh.valuation)
        .await
        .unwrap();
    assert!(harness.targets.guardrails()[0].is_outside());

    harness.holdings.clear_holdings();
    harness.seed_holding("a", "5");
    harness.seed_holding("b", "5");
    let fresh = harness.valuation.fresh(&portfolio).await.unwrap();
    harness
        .guardrails
        .evaluate(&portfolio, &fresh.valuation)
        .await
        .unwrap();
    assert!(!harness.targets.guardrails()[0].is_outside());
}

#[tokio::test]
async fn the_run_writes_one_snapshot_per_portfolio() {
    let harness = harness(at(0), None);
    harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let report = harness.run.run_scheduled("test").await.unwrap();
    assert_eq!(report.portfolios, 1);
    assert_eq!(report.snapshots, 1);
    assert_eq!(harness.snapshots.all().len(), 1);
}

#[tokio::test]
async fn a_second_run_in_the_same_hour_skips_the_snapshot() {
    let harness = harness(at(0), None);
    harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    harness.run.run_scheduled("test").await.unwrap();
    let report = harness.run.run_scheduled("test").await.unwrap();
    assert_eq!(report.snapshots, 0);
    assert_eq!(report.skipped, 1);
    assert_eq!(harness.snapshots.all().len(), 1);
}

#[tokio::test]
async fn a_held_lease_makes_the_run_conflict() {
    let harness = harness(at(0), None);
    let held = ValuationRun {
        id: uuid::Uuid::new_v4(),
        job: portfolio_application::run::RUN_JOB.to_string(),
        owner: "other".to_string(),
        started_at: at(0),
        lease_until: at(0) + chrono::Duration::seconds(900),
        finished_at: None,
        status: RunStatus::Running,
        detail: None,
    };
    harness.runs.claim(&held).await.unwrap();
    let error = harness
        .run
        .run(&service("job-orchestration"), "test")
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Conflict(_)));
}

#[tokio::test]
async fn an_expired_lease_is_swept_and_the_run_proceeds() {
    let harness = harness(at(0), None);
    let expired = ValuationRun {
        id: uuid::Uuid::new_v4(),
        job: portfolio_application::run::RUN_JOB.to_string(),
        owner: "old".to_string(),
        started_at: at(0) - chrono::Duration::seconds(1800),
        lease_until: at(0) - chrono::Duration::seconds(900),
        finished_at: None,
        status: RunStatus::Running,
        detail: None,
    };
    harness.runs.claim(&expired).await.unwrap();
    let report = harness.run.run_scheduled("test").await.unwrap();
    assert_eq!(report.portfolios, 0);
    let rows = harness.runs.all();
    assert!(rows
        .iter()
        .any(|row| row.id == expired.id && row.status == RunStatus::Failed));
    assert!(rows
        .iter()
        .any(|row| row.owner == "test" && row.status == RunStatus::Succeeded));
}

#[tokio::test]
async fn the_run_removes_an_overlay_c5_now_reports() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let trade = harness
        .trades
        .record(&caller(), portfolio.id(), &trade_request("idem-1"))
        .await
        .unwrap();
    assert_eq!(harness.overlays.all().len(), 1);
    harness.holdings.seed_transaction(
        account(),
        VenueTradeObservation {
            external_id: Some("venue-1".to_string()),
            asset_id: "a".to_string(),
            side: OrderSide::Buy,
            quantity: d("1"),
            executed_at: trade.executed_at(),
        },
    );
    harness.run.run_scheduled("test").await.unwrap();
    assert!(harness.overlays.all().is_empty());
}

#[tokio::test]
async fn the_run_keeps_an_overlay_c5_does_not_report() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    harness
        .trades
        .record(&caller(), portfolio.id(), &trade_request("idem-1"))
        .await
        .unwrap();
    harness.holdings.seed_transaction(
        account(),
        VenueTradeObservation {
            external_id: Some("venue-1".to_string()),
            asset_id: "a".to_string(),
            side: OrderSide::Buy,
            quantity: d("9"),
            executed_at: at(2),
        },
    );
    harness.run.run_scheduled("test").await.unwrap();
    assert_eq!(harness.overlays.all().len(), 1);
}

#[tokio::test]
async fn the_run_requires_the_run_operation() {
    let harness = harness(at(0), None);
    let error = harness
        .run
        .run(&service("identity"), "test")
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
}

#[tokio::test]
async fn privacy_export_gathers_the_portfolio_data() {
    let harness = harness(at(0), None);
    harness.seed_portfolio(user_a());
    let export = harness
        .privacy
        .export(&service("identity"), user_a())
        .await
        .unwrap();
    assert_eq!(export.portfolios.len(), 1);
    assert_eq!(export.memberships.len(), 1);
}

#[tokio::test]
async fn privacy_export_requires_its_operation() {
    let harness = harness(at(0), None);
    let error = harness
        .privacy
        .export(&service("job-orchestration"), user_a())
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
}

#[tokio::test]
async fn privacy_anonymise_applies_once() {
    let harness = harness(at(0), None);
    harness.seed_portfolio(user_a());
    let first = harness
        .privacy
        .anonymise(&service("identity"), user_a(), "idem-1")
        .await
        .unwrap();
    assert!(!first.already_applied);
    assert!(harness.portfolios_repo.all()[0].anonymised_at().is_some());
    let second = harness
        .privacy
        .anonymise(&service("identity"), user_a(), "idem-1")
        .await
        .unwrap();
    assert!(second.already_applied);
    assert!(harness
        .audit
        .records()
        .iter()
        .any(|record| record.action() == "system.privacy.deletion"));
}

#[tokio::test]
async fn privacy_anonymise_erases_the_export_by_the_original_id() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    harness
        .trades
        .record(&caller(), portfolio.id(), &trade_request("idem-trade"))
        .await
        .unwrap();
    assert_eq!(
        harness
            .privacy
            .export(&service("identity"), user_a())
            .await
            .unwrap()
            .portfolios
            .len(),
        1
    );

    harness
        .privacy
        .anonymise(&service("identity"), user_a(), "idem-erase")
        .await
        .unwrap();

    let after = harness
        .privacy
        .export(&service("identity"), user_a())
        .await
        .unwrap();
    assert!(after.portfolios.is_empty());
    assert!(after.memberships.is_empty());
    assert!(after.snapshots.is_empty());
    assert!(after.trades.is_empty());
    // The subject column now holds a random opaque reference, not the original.
    let stored = harness.portfolios_repo.all();
    assert_eq!(stored.len(), 1);
    assert_ne!(stored[0].user_id(), user_a());
    let trades = harness.trades_repo.all();
    assert_eq!(trades.len(), 1);
    assert_ne!(trades[0].user_id(), user_a());
}

#[tokio::test]
async fn privacy_anonymise_requires_an_idempotency_key() {
    let harness = harness(at(0), None);
    let error = harness
        .privacy
        .anonymise(&service("identity"), user_a(), "  ")
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        AppError::Validation(errors) if errors[0].field == "idempotencyKey"
    ));
}

#[tokio::test]
async fn the_run_reports_a_failed_portfolio_without_aborting_the_page() {
    let harness = harness(at(0), None);
    harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.holdings.set_unavailable(true);
    let report = harness.run.run_scheduled("test").await.unwrap();
    assert_eq!(report.portfolios, 1);
    assert_eq!(report.failed, 1);
    assert_eq!(report.snapshots, 0);
}

#[tokio::test]
async fn an_empty_database_run_succeeds_with_no_work() {
    let harness = harness(at(0), None);
    let report = harness.run.run_scheduled("test").await.unwrap();
    assert_eq!(report.portfolios, 0);
    assert_eq!(report.failed, 0);
}

#[tokio::test]
async fn the_service_allow_list_is_enforced_for_snapshots() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    let views = harness
        .valuation
        .history_for_service(&service("reporting"), portfolio.id(), None, 100, None)
        .await
        .unwrap();
    assert!(views.snapshots.is_empty());
    assert!(matches!(
        harness
            .valuation
            .history_for_service(&service("identity"), portfolio.id(), None, 100, None)
            .await
            .unwrap_err(),
        AppError::Forbidden(_)
    ));
}

#[tokio::test]
async fn an_unavailable_write_is_surfaced() {
    let harness = harness(at(0), None);
    harness.portfolios_repo.fail_writes(true);
    let error = harness
        .portfolios
        .list_or_create(&caller())
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Unavailable(_)));
}

#[tokio::test]
async fn reading_a_valuation_applies_a_post_snapshot_overlay() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let fresh = harness.valuation.fresh(&portfolio).await.unwrap();
    harness
        .valuation
        .store_snapshot(&portfolio, &fresh.valuation, &fresh.inputs, at(0))
        .await
        .unwrap();
    // The trade lands 30 s after the snapshot, still inside the 60 s serve window.
    harness.clock.set(at(0) + chrono::Duration::seconds(30));
    harness
        .trades
        .record(&caller(), portfolio.id(), &trade_request("idem-1"))
        .await
        .unwrap();
    // Only the snapshot path can answer once C5 is down.
    harness.holdings.set_unavailable(true);
    let read = harness
        .valuation
        .read(&caller(), portfolio.id())
        .await
        .unwrap();
    assert!(read.served_from_snapshot);
    assert_eq!(read.valuation.total_value(), Some(d("200")));
    assert_eq!(read.as_of, at(0) + chrono::Duration::seconds(30));
}

#[tokio::test]
async fn an_intent_preview_uses_a_stale_snapshot_without_c5_or_c6() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let fresh = harness.valuation.fresh(&portfolio).await.unwrap();
    harness
        .valuation
        .store_snapshot(&portfolio, &fresh.valuation, &fresh.inputs, at(0))
        .await
        .unwrap();
    harness.clock.set(at(2));
    harness.holdings.set_unavailable(true);
    harness.market.set_unavailable(true);
    let projection = harness
        .intents
        .preview(
            &caller(),
            portfolio.id(),
            &IntentRequest {
                asset_id: "a".to_string(),
                side: OrderSide::Buy,
                quantity: d("1"),
            },
        )
        .await
        .unwrap();
    assert_eq!(projection.projected_allocation, d("100"));
}

#[tokio::test]
async fn history_is_paginated_and_reproduction_is_on_demand() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    for hour in 0..3 {
        let fresh = harness.valuation.fresh(&portfolio).await.unwrap();
        harness
            .valuation
            .store_snapshot(&portfolio, &fresh.valuation, &fresh.inputs, at(hour))
            .await
            .unwrap();
    }
    let first = harness
        .valuation
        .history(portfolio.id(), None, 2, None)
        .await
        .unwrap();
    assert_eq!(first.snapshots.len(), 2);
    assert!(first.has_more);
    let cursor = first.next_cursor.unwrap();
    let second = harness
        .valuation
        .history(portfolio.id(), None, 2, Some(cursor))
        .await
        .unwrap();
    assert_eq!(second.snapshots.len(), 1);
    assert!(!second.has_more);
}

#[tokio::test]
async fn the_run_records_a_failed_portfolio_and_continues() {
    let harness = harness(at(0), None);
    for index in 0..4 {
        let account_id = AccountId::new(uuid::Uuid::from_u128(100 + index));
        harness.seed_portfolio_with_account(user_a(), account_id);
        harness.seed_holding_for(account_id, "a", "1");
    }
    let failed_account = AccountId::new(uuid::Uuid::from_u128(200));
    harness.seed_portfolio_with_account(user_b(), failed_account);
    harness.seed_quote("a", "100", "USD");
    harness.holdings.set_user_unavailable(user_b(), true);
    let report = harness.run.run_scheduled("test").await.unwrap();
    assert_eq!(report.portfolios, 5);
    assert_eq!(report.failed, 1);
    assert_eq!(report.snapshots, 4);
}

#[tokio::test]
async fn overlay_reconciliation_accepts_quantity_and_time_tolerance() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let trade = harness
        .trades
        .record(&caller(), portfolio.id(), &trade_request("idem-1"))
        .await
        .unwrap();
    harness.holdings.seed_transaction(
        account(),
        VenueTradeObservation {
            external_id: Some("venue-1".to_string()),
            asset_id: "a".to_string(),
            side: OrderSide::Buy,
            quantity: d("1.0005"),
            executed_at: trade.executed_at() + chrono::Duration::hours(10),
        },
    );
    harness.run.run_scheduled("test").await.unwrap();
    assert!(harness.overlays.all().is_empty());
}

#[tokio::test]
async fn overlay_reconciliation_rejects_a_transaction_beyond_24_hours() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    let trade = harness
        .trades
        .record(&caller(), portfolio.id(), &trade_request("idem-1"))
        .await
        .unwrap();
    harness.holdings.seed_transaction(
        account(),
        VenueTradeObservation {
            external_id: Some("venue-1".to_string()),
            asset_id: "a".to_string(),
            side: OrderSide::Buy,
            quantity: d("1"),
            executed_at: trade.executed_at() + chrono::Duration::hours(25),
        },
    );
    harness.run.run_scheduled("test").await.unwrap();
    assert_eq!(harness.overlays.all().len(), 1);
}

#[tokio::test]
async fn the_run_expires_an_unmatched_overlay_after_seven_days() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "1");
    harness.seed_quote("a", "100", "USD");
    harness
        .trades
        .record(&caller(), portfolio.id(), &trade_request("idem-1"))
        .await
        .unwrap();
    assert_eq!(harness.overlays.all().len(), 1);
    harness.clock.set(at(0) + chrono::Duration::days(8));
    let report = harness.run.run_scheduled("test").await.unwrap();
    assert_eq!(report.overlays_expired, 1);
    assert!(harness.overlays.all().is_empty());
}

#[tokio::test]
async fn saving_targets_preserves_an_unchanged_guardrail_crossing() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    let previous = Guardrail::from_stored(
        GuardrailId::generate(),
        portfolio.id(),
        "a".to_string(),
        d("5"),
        true,
        Some("drift:existing".to_string()),
        Some(at(1)),
        at(1),
    );
    harness.targets.seed_guardrail(previous);
    harness
        .targets_service
        .save(
            &caller(),
            portfolio.id(),
            &[TargetInput {
                asset_id: "a".to_string(),
                target_percent: d("50"),
                guardrail_points: Some(d("5")),
            }],
        )
        .await
        .unwrap();
    let saved = harness.targets.guardrails();
    assert_eq!(saved.len(), 1);
    assert!(saved[0].is_outside());
    assert_eq!(saved[0].last_crossing_key(), Some("drift:existing"));
}

#[tokio::test]
async fn a_guardrail_alert_uses_only_allow_listed_payload_fields() {
    let harness = harness(at(0), None);
    let portfolio = harness.seed_portfolio(user_a());
    harness.seed_holding("a", "6");
    harness.seed_holding("b", "4");
    harness.seed_quote("a", "100", "USD");
    harness.seed_quote("b", "100", "USD");
    harness.seed_target(&portfolio, "a", "50");
    harness.seed_guardrail(&portfolio, "a", "5");
    let fresh = harness.valuation.fresh(&portfolio).await.unwrap();
    harness
        .guardrails
        .evaluate(&portfolio, &fresh.valuation)
        .await
        .unwrap();
    let alerts = harness.notifications.alerts();
    assert_eq!(alerts.len(), 1);
    let payload = alerts[0].3.as_object().unwrap();
    assert_eq!(payload.len(), 3);
    assert!(payload.contains_key("portfolioId"));
    assert!(payload.contains_key("assetId"));
    assert!(payload.contains_key("driftPoints"));
    assert!(!payload.contains_key("guardrailPoints"));
}

fn trade_request(key: &str) -> TradeRequest {
    TradeRequest {
        asset_id: "a".to_string(),
        side: OrderSide::Buy,
        quantity: d("1"),
        price: Some(d("100")),
        price_source: PriceSource::User,
        executed_at: at(1),
        idempotency_key: key.to_string(),
    }
}
