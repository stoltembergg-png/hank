use auth_core::callback::{CallbackError, CallbackUrl, OAuthCallbackHandler};
use auth_core::{
    AuthorizationCode, CodeChallenge, OAuthError, OAuthFlowContext, OAuthState, PkceVerifier,
    RedirectUri, TokenExchangeBackend,
};
use provider_core::credentials::{
    AccountId, CredentialAccessContext, CredentialAccount, ProjectScopeId,
};
use provider_core::{CancellationToken, CredentialRef, ProviderId};

struct MockExchange;

impl TokenExchangeBackend for MockExchange {
    fn exchange(
        &self,
        _provider_id: &ProviderId,
        _code: AuthorizationCode,
        _verifier: PkceVerifier,
    ) -> Result<CredentialRef, OAuthError> {
        Ok(CredentialRef::parse("cred_openai_1").unwrap())
    }
}

fn account(project: &str, provider: &str, id: &str) -> CredentialAccount {
    CredentialAccount::new(
        ProjectScopeId::parse(project).unwrap(),
        ProviderId::parse(provider).unwrap(),
        AccountId::parse(id).unwrap(),
    )
    .unwrap()
}

fn access(project: &str) -> CredentialAccessContext {
    CredentialAccessContext::new(
        ProjectScopeId::parse(project).unwrap(),
        "agent_1".into(),
        CancellationToken::new(),
    )
    .unwrap()
}

fn flow_context(now: u64, deadline: u64) -> OAuthFlowContext {
    OAuthFlowContext::new(now, deadline, CancellationToken::new()).unwrap()
}

fn verifier() -> PkceVerifier {
    PkceVerifier::parse("verifier_abcdefghijklmnopqrstuvwxyz0123456789").unwrap()
}

fn challenge() -> CodeChallenge {
    CodeChallenge::from_verifier(&verifier())
}

fn start(handler: &OAuthCallbackHandler<MockExchange>) -> auth_core::AuthorizationRequest {
    handler
        .begin(
            account("project_1", "openai", "account_1"),
            RedirectUri::parse("http://localhost:1420/oauth/callback").unwrap(),
            OAuthState::parse("state_abc123").unwrap(),
            challenge(),
            access("project_1"),
            flow_context(1_000, 2_000),
        )
        .unwrap()
}

fn callback_url(
    flow: &auth_core::AuthorizationRequest,
    provider: &str,
    account: &str,
    state: &str,
) -> String {
    format!(
        "hank://oauth/callback?flow={}&provider={}&account={}&state={}&code=code_opaque",
        flow.flow_id.as_str(),
        provider,
        account,
        state
    )
}

#[test]
fn valid_callback_yields_opaque_credential_result() {
    let handler = OAuthCallbackHandler::new(MockExchange);
    let flow = start(&handler);
    let result = handler
        .complete(
            &callback_url(&flow, "openai", "account_1", "state_abc123"),
            access("project_1"),
            flow_context(1_500, 2_000),
            verifier(),
        )
        .unwrap();
    assert_eq!(result.provider_id().as_str(), "openai");
    assert_eq!(result.account_id().as_str(), "account_1");
    assert_eq!(result.credential_ref.as_str(), "cred_openai_1");
}

#[test]
fn malformed_and_foreign_callbacks_fail_closed() {
    let handler = OAuthCallbackHandler::new(MockExchange);
    let flow = start(&handler);
    assert!(matches!(
        handler.complete(
            "https://evil.example/callback",
            access("project_1"),
            flow_context(1_500, 2_000),
            verifier()
        ),
        Err(CallbackError::Malformed)
    ));
    assert!(matches!(
        handler.complete(
            &callback_url(&flow, "anthropic", "account_1", "state_abc123"),
            access("project_1"),
            flow_context(1_500, 2_000),
            verifier()
        ),
        Err(CallbackError::ProviderMismatch)
    ));
    assert!(matches!(
        handler.complete(
            &callback_url(&flow, "openai", "account_2", "state_abc123"),
            access("project_1"),
            flow_context(1_500, 2_000),
            verifier()
        ),
        Err(CallbackError::AccountMismatch)
    ));
}

#[test]
fn wrong_project_and_state_are_rejected() {
    let handler = OAuthCallbackHandler::new(MockExchange);
    let flow = start(&handler);
    assert!(matches!(
        handler.complete(
            &callback_url(&flow, "openai", "account_1", "state_abc123"),
            access("project_2"),
            flow_context(1_500, 2_000),
            verifier()
        ),
        Err(CallbackError::Unauthorized)
    ));
    assert!(matches!(
        handler.complete(
            &callback_url(&flow, "openai", "account_1", "state_wrong"),
            access("project_1"),
            flow_context(1_500, 2_000),
            verifier()
        ),
        Err(CallbackError::OAuth(OAuthError::StateMismatch))
    ));
}

#[test]
fn callback_is_consumed_exactly_once_and_timeout_is_terminal() {
    let handler = OAuthCallbackHandler::new(MockExchange);
    let flow = start(&handler);
    let url = callback_url(&flow, "openai", "account_1", "state_abc123");
    handler
        .complete(
            &url,
            access("project_1"),
            flow_context(1_500, 2_000),
            verifier(),
        )
        .unwrap();
    assert!(matches!(
        handler.complete(
            &url,
            access("project_1"),
            flow_context(1_500, 2_000),
            verifier()
        ),
        Err(CallbackError::OAuth(OAuthError::Replay))
    ));

    let handler = OAuthCallbackHandler::new(MockExchange);
    let flow = start(&handler);
    assert!(matches!(
        handler.complete(
            &callback_url(&flow, "openai", "account_1", "state_abc123"),
            access("project_1"),
            flow_context(2_001, 3_000),
            verifier()
        ),
        Err(CallbackError::OAuth(OAuthError::Expired))
    ));
}

#[test]
fn callback_parser_rejects_duplicates_unknowns_and_unbounded_values() {
    assert!(CallbackUrl::parse("hank://oauth/callback?flow=flow_1&provider=openai&account=account_1&state=state_abc123&code=code_opaque").is_ok());
    assert!(CallbackUrl::parse("hank://oauth/callback?flow=flow_1&flow=flow_2&provider=openai&account=account_1&state=state_abc123&code=code_opaque").is_err());
    assert!(CallbackUrl::parse("hank://oauth/callback?flow=flow_1&provider=openai&account=account_1&state=state_abc123&unknown=x&code=code_opaque").is_err());
    assert!(CallbackUrl::parse("hank://oauth/other?flow=flow_1&provider=openai&account=account_1&state=state_abc123&code=code_opaque").is_err());
}

#[test]
fn callback_errors_and_debug_do_not_expose_code() {
    let handler = OAuthCallbackHandler::new(MockExchange);
    let error = handler
        .complete("hank://oauth/callback?flow=flow_1&provider=openai&account=account_1&state=state_abc123&code=secret-code", access("project_1"), flow_context(1_500, 2_000), verifier())
        .unwrap_err();
    assert!(!error.to_string().contains("secret-code"));
    assert!(!format!("{error:?}").contains("secret-code"));
}
