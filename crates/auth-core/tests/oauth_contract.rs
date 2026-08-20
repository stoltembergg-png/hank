use auth_core::{
    AuthorizationCode, CodeChallenge, OAuthCallback, OAuthError, OAuthFlowContext,
    OAuthFlowManager, OAuthState, PkceVerifier, RedirectUri, TokenExchangeBackend,
};
use provider_core::{CancellationToken, CredentialRef, ProviderId};

struct MockExchange {
    result: Result<CredentialRef, OAuthError>,
}

impl TokenExchangeBackend for MockExchange {
    fn exchange(
        &self,
        _provider_id: &ProviderId,
        _code: AuthorizationCode,
        _verifier: PkceVerifier,
    ) -> Result<CredentialRef, OAuthError> {
        self.result.clone()
    }
}

fn context(now_ms: u64, deadline_ms: u64) -> OAuthFlowContext {
    OAuthFlowContext::new(now_ms, deadline_ms, CancellationToken::new()).unwrap()
}

fn redirect() -> RedirectUri {
    RedirectUri::parse("http://localhost:1420/oauth/callback").unwrap()
}

fn state() -> OAuthState {
    OAuthState::parse("state_abc123").unwrap()
}

fn challenge() -> CodeChallenge {
    CodeChallenge::from_verifier(&verifier())
}

fn verifier() -> PkceVerifier {
    PkceVerifier::parse("verifier_abcdefghijklmnopqrstuvwxyz0123456789").unwrap()
}

#[test]
fn begin_creates_exact_redirect_pkce_and_state_bound_request() {
    let manager = OAuthFlowManager::new(MockExchange {
        result: Ok(CredentialRef::parse("cred_openai_1").unwrap()),
    });
    let request = manager
        .begin(
            ProviderId::parse("openai").unwrap(),
            redirect(),
            state(),
            challenge(),
            context(1_000, 2_000),
        )
        .unwrap();
    assert_eq!(
        request.redirect_uri.as_str(),
        "http://localhost:1420/oauth/callback"
    );
    assert_eq!(request.state.as_str(), "state_abc123");
    assert_eq!(request.code_challenge, challenge());
    assert_eq!(request.expires_at_ms, 2_000);
}

#[test]
fn callback_exchange_handoffs_only_opaque_credential_ref() {
    let manager = OAuthFlowManager::new(MockExchange {
        result: Ok(CredentialRef::parse("cred_openai_1").unwrap()),
    });
    let request = manager
        .begin(
            ProviderId::parse("openai").unwrap(),
            redirect(),
            state(),
            challenge(),
            context(1_000, 2_000),
        )
        .unwrap();
    let result = manager
        .complete(
            request.flow_id,
            OAuthCallback::new(
                request.state,
                request.redirect_uri,
                AuthorizationCode::parse("code_opaque").unwrap(),
            )
            .unwrap(),
            verifier(),
            context(1_500, 2_000),
        )
        .unwrap();
    assert_eq!(result.as_str(), "cred_openai_1");
}

#[test]
fn wrong_state_redirect_and_replay_fail_closed() {
    let manager = OAuthFlowManager::new(MockExchange {
        result: Ok(CredentialRef::parse("cred_openai_1").unwrap()),
    });
    let request = manager
        .begin(
            ProviderId::parse("openai").unwrap(),
            redirect(),
            state(),
            challenge(),
            context(1_000, 2_000),
        )
        .unwrap();
    let wrong_state = OAuthCallback::new(
        OAuthState::parse("state_wrong").unwrap(),
        request.redirect_uri.clone(),
        AuthorizationCode::parse("code_opaque").unwrap(),
    )
    .unwrap();
    assert!(matches!(
        manager.complete(
            request.flow_id,
            wrong_state,
            verifier(),
            context(1_500, 2_000)
        ),
        Err(OAuthError::StateMismatch)
    ));

    let wrong_redirect = OAuthCallback::new(
        request.state.clone(),
        RedirectUri::parse("http://localhost:1420/other").unwrap(),
        AuthorizationCode::parse("code_opaque").unwrap(),
    )
    .unwrap();
    assert!(matches!(
        manager.complete(
            request.flow_id,
            wrong_redirect,
            verifier(),
            context(1_500, 2_000)
        ),
        Err(OAuthError::RedirectMismatch)
    ));

    let callback = OAuthCallback::new(
        request.state,
        request.redirect_uri,
        AuthorizationCode::parse("code_opaque").unwrap(),
    )
    .unwrap();
    manager
        .complete(
            request.flow_id,
            callback.clone(),
            verifier(),
            context(1_500, 2_000),
        )
        .unwrap();
    assert!(matches!(
        manager.complete(request.flow_id, callback, verifier(), context(1_500, 2_000)),
        Err(OAuthError::Replay)
    ));
}

#[test]
fn expired_and_cancelled_flows_fail_closed() {
    let manager = OAuthFlowManager::new(MockExchange {
        result: Ok(CredentialRef::parse("cred_openai_1").unwrap()),
    });
    let request = manager
        .begin(
            ProviderId::parse("openai").unwrap(),
            redirect(),
            state(),
            challenge(),
            context(1_000, 2_000),
        )
        .unwrap();
    let callback = OAuthCallback::new(
        request.state,
        request.redirect_uri,
        AuthorizationCode::parse("code_opaque").unwrap(),
    )
    .unwrap();
    assert!(matches!(
        manager.complete(
            request.flow_id,
            callback.clone(),
            verifier(),
            context(2_001, 3_000)
        ),
        Err(OAuthError::Expired)
    ));

    let token = CancellationToken::new();
    token.cancel();
    let cancelled = OAuthFlowContext::new(1_500, 2_000, token).unwrap();
    assert!(matches!(
        manager.complete(request.flow_id, callback, verifier(), cancelled),
        Err(OAuthError::Cancelled)
    ));
}

#[test]
fn malformed_token_and_invalid_redirect_are_rejected() {
    let manager = OAuthFlowManager::new(MockExchange {
        result: Err(OAuthError::MalformedToken),
    });
    let request = manager
        .begin(
            ProviderId::parse("openai").unwrap(),
            redirect(),
            state(),
            challenge(),
            context(1_000, 2_000),
        )
        .unwrap();
    let callback = OAuthCallback::new(
        request.state,
        request.redirect_uri,
        AuthorizationCode::parse("code_opaque").unwrap(),
    )
    .unwrap();
    assert!(matches!(
        manager.complete(request.flow_id, callback, verifier(), context(1_500, 2_000)),
        Err(OAuthError::MalformedToken)
    ));
    assert!(RedirectUri::parse("https://example.com/callback?x=1").is_ok());
    assert!(RedirectUri::parse("http://remote/callback").is_err());
    assert!(PkceVerifier::parse("short").is_err());
}

#[test]
fn sensitive_wrappers_are_redacted() {
    let code = AuthorizationCode::parse("code_opaque").unwrap();
    let verifier = PkceVerifier::parse("verifier_abcdefghijklmnopqrstuvwxyz0123456789").unwrap();
    assert!(!format!("{code:?}").contains("code_opaque"));
    assert!(!format!("{verifier:?}").contains("verifier_"));
}
