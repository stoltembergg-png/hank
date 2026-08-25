use serde_json::json;
use workflow_core::condition::{ConditionError, ConditionExpression, ConditionValue};

// @spec:AC-989
#[test]
fn parser_accepts_typed_comparisons_and_rejects_code() {
    let expression = ConditionExpression::parse("$.score > 10").unwrap();
    assert_eq!(expression.operator(), ">");
    assert!(matches!(
        ConditionExpression::parse("eval(process())"),
        Err(ConditionError::UnsupportedSyntax)
    ));
    assert!(matches!(
        ConditionExpression::parse("$.x == 1; delete"),
        Err(ConditionError::UnsupportedSyntax)
    ));
}

// @spec:AC-990
#[test]
fn evaluation_is_deterministic_and_side_effect_free() {
    let expression = ConditionExpression::parse("$.status == \"ready\"").unwrap();
    let input = json!({"status":"ready"});
    assert_eq!(expression.evaluate(&input).unwrap(), ConditionValue::True);
    assert_eq!(expression.evaluate(&input).unwrap(), ConditionValue::True);
}

// @spec:AC-991
#[test]
fn missing_fields_depth_and_type_mismatch_fail_closed() {
    let missing = ConditionExpression::parse("$.missing == true").unwrap();
    assert!(matches!(
        missing.evaluate(&json!({})),
        Err(ConditionError::UnknownField)
    ));
    let mismatch = ConditionExpression::parse("$.status > 10").unwrap();
    assert!(matches!(
        mismatch.evaluate(&json!({"status":"ready"})),
        Err(ConditionError::TypeMismatch)
    ));
    let deep = format!(
        "$.{} == true",
        (0..20).map(|_| "nested").collect::<Vec<_>>().join(".")
    );
    assert!(matches!(
        ConditionExpression::parse(&deep),
        Err(ConditionError::DepthExceeded)
    ));
}
