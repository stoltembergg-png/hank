use provider_core::credentials::{
    AccountId, CredentialAccessContext, CredentialAccount, CredentialService,
    CredentialServiceError, CredentialServiceState, InMemoryCredentialService, ProjectScopeId,
};
use provider_core::{CancellationToken, CredentialRef, ProviderId};

fn context(project: &str, actor: &str) -> CredentialAccessContext {
    CredentialAccessContext::new(
        ProjectScopeId::parse(project).unwrap(),
        actor.to_string(),
        CancellationToken::new(),
    )
    .unwrap()
}

fn account(project: &str, provider: &str, account: &str) -> CredentialAccount {
    CredentialAccount::new(
        ProjectScopeId::parse(project).unwrap(),
        ProviderId::parse(provider).unwrap(),
        AccountId::parse(account).unwrap(),
    )
    .unwrap()
}

#[test]
fn credential_connect_returns_status_and_opaque_ref() {
    let service = InMemoryCredentialService::new();
    let account = account("project_1", "openai", "account_1");
    let result = service
        .connect(
            context("project_1", "agent_1"),
            account.clone(),
            CredentialRef::parse("cred_openai_1").unwrap(),
        )
        .unwrap();

    assert_eq!(result.account, account);
    assert_eq!(result.state, CredentialServiceState::Connected);
    assert_eq!(result.credential_ref.unwrap().as_str(), "cred_openai_1");
}

#[test]
fn unauthorized_cross_project_access_is_rejected() {
    let service = InMemoryCredentialService::new();
    let account = account("project_1", "openai", "account_1");
    service
        .connect(
            context("project_1", "agent_1"),
            account.clone(),
            CredentialRef::parse("cred_openai_1").unwrap(),
        )
        .unwrap();

    let err = service
        .resolve_ref(context("project_2", "agent_1"), account)
        .unwrap_err();
    assert!(matches!(err, CredentialServiceError::Unauthorized));
    assert!(!err.to_string().contains("cred_openai_1"));
}

#[test]
fn resolve_returns_ref_only_while_connected() {
    let service = InMemoryCredentialService::new();
    let account = account("project_1", "anthropic", "account_1");
    let ctx = context("project_1", "agent_1");
    service
        .connect(
            ctx.clone(),
            account.clone(),
            CredentialRef::parse("cred_anthropic_1").unwrap(),
        )
        .unwrap();

    assert_eq!(
        service
            .resolve_ref(ctx.clone(), account.clone())
            .unwrap()
            .as_str(),
        "cred_anthropic_1"
    );
    service.disconnect(ctx.clone(), account.clone()).unwrap();
    assert!(matches!(
        service.resolve_ref(ctx, account),
        Err(CredentialServiceError::Revoked)
    ));
}

#[test]
fn malformed_plaintext_secret_is_rejected_before_service() {
    assert!(CredentialRef::parse("api_key=plaintext").is_err());
    assert!(CredentialRef::parse("token_plaintext").is_err());
}

#[test]
fn unavailable_state_fails_closed_without_plaintext_fallback() {
    let service = InMemoryCredentialService::unavailable();
    let account = account("project_1", "openai", "account_1");
    let err = service
        .connect(
            context("project_1", "agent_1"),
            account,
            CredentialRef::parse("cred_openai_1").unwrap(),
        )
        .unwrap_err();
    assert!(matches!(err, CredentialServiceError::Unavailable));
}

#[test]
fn cancellation_is_explicit() {
    let service = InMemoryCredentialService::new();
    let token = CancellationToken::new();
    token.cancel();
    let ctx = CredentialAccessContext::new(
        ProjectScopeId::parse("project_1").unwrap(),
        "agent_1".into(),
        token,
    )
    .unwrap();
    let err = service
        .connect(
            ctx,
            account("project_1", "openai", "account_1"),
            CredentialRef::parse("cred_openai_1").unwrap(),
        )
        .unwrap_err();
    assert!(matches!(err, CredentialServiceError::Cancelled));
}

#[test]
fn account_identity_and_status_are_bounded_and_deterministic() {
    let account = account("project_1", "openai", "account_1");
    assert_eq!(account.provider_id.as_str(), "openai");
    assert_eq!(account.account_id.as_str(), "account_1");
    assert_eq!(account.project_id.as_str(), "project_1");
    let rendered = format!("{account:?}");
    assert!(!rendered.contains("cred_"));
}

#[test]
fn missing_account_and_duplicate_connection_are_typed() {
    let service = InMemoryCredentialService::new();
    let account = account("project_1", "openai", "account_1");
    let ctx = context("project_1", "agent_1");
    assert!(matches!(
        service.status(ctx.clone(), account.clone()),
        Err(CredentialServiceError::Missing)
    ));
    service
        .connect(
            ctx.clone(),
            account.clone(),
            CredentialRef::parse("cred_openai_1").unwrap(),
        )
        .unwrap();
    assert!(matches!(
        service.connect(ctx, account, CredentialRef::parse("cred_openai_2").unwrap(),),
        Err(CredentialServiceError::Conflict)
    ));
}

#[test]
fn concurrent_status_reads_are_safe() {
    use std::sync::Arc;
    use std::thread;

    let service = Arc::new(InMemoryCredentialService::new());
    let account = account("project_1", "openai", "account_1");
    service
        .connect(
            context("project_1", "agent_1"),
            account.clone(),
            CredentialRef::parse("cred_openai_1").unwrap(),
        )
        .unwrap();
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let service = Arc::clone(&service);
            let account = account.clone();
            thread::spawn(move || {
                for _ in 0..100 {
                    assert_eq!(
                        service
                            .status(context("project_1", "agent_1"), account.clone())
                            .unwrap()
                            .state,
                        CredentialServiceState::Connected
                    );
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
}
