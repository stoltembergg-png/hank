use remote_adapter::{new_credential_broker, OsEntropy};
use remote_core::credential_broker::BrokerEntropy;

#[test]
// @spec:AC-1466
fn os_entropy_produces_independent_seeds() {
    let first = OsEntropy.next_seed().expect("OS CSPRNG must be available");
    let second = OsEntropy.next_seed().expect("OS CSPRNG must be available");
    assert_ne!(first, second, "independent OS entropy draws must differ");
}

#[test]
fn production_constructor_is_fallible() {
    let result = new_credential_broker();
    assert!(result.is_ok());
}
