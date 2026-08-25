//! Deterministic, side-effect-free evaluator for the ConditionNode subset.

use serde_json::Value;
use thiserror::Error;

const MAX_EXPRESSION_BYTES: usize = 256;
const MAX_PATH_DEPTH: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionValue {
    True,
    False,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operator {
    Eq,
    Ne,
    Gt,
    Lt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConditionExpression {
    path: Vec<String>,
    operator: Operator,
    literal: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ConditionError {
    #[error("condition expression syntax is unsupported")]
    UnsupportedSyntax,
    #[error("condition expression exceeds depth limit")]
    DepthExceeded,
    #[error("condition path field is unknown")]
    UnknownField,
    #[error("condition operand types are incompatible")]
    TypeMismatch,
    #[error("condition literal is invalid")]
    InvalidLiteral,
}

impl ConditionExpression {
    pub fn parse(source: &str) -> Result<Self, ConditionError> {
        if source.is_empty()
            || source.len() > MAX_EXPRESSION_BYTES
            || source.chars().any(char::is_control)
        {
            return Err(ConditionError::UnsupportedSyntax);
        }
        let (path_text, operator, literal_text) = ["==", "!=", ">", "<"]
            .iter()
            .find_map(|token| {
                source
                    .split_once(token)
                    .map(|(left, right)| (left.trim(), *token, right.trim()))
            })
            .ok_or(ConditionError::UnsupportedSyntax)?;
        if path_text.is_empty()
            || literal_text.is_empty()
            || literal_text.contains(';')
            || literal_text.contains('(')
            || literal_text.contains(')')
        {
            return Err(ConditionError::UnsupportedSyntax);
        }
        if !path_text.starts_with("$.")
            || path_text[2..]
                .contains(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
        {
            return Err(ConditionError::UnsupportedSyntax);
        }
        let path: Vec<String> = path_text[2..].split('.').map(str::to_string).collect();
        if path.is_empty() || path.iter().any(|part| part.is_empty()) {
            return Err(ConditionError::UnsupportedSyntax);
        }
        if path.len() > MAX_PATH_DEPTH {
            return Err(ConditionError::DepthExceeded);
        }
        let literal: Value =
            serde_json::from_str(literal_text).map_err(|_| ConditionError::InvalidLiteral)?;
        if !literal.is_null()
            && !literal.is_boolean()
            && !literal.is_number()
            && !literal.is_string()
        {
            return Err(ConditionError::InvalidLiteral);
        }
        let operator = match operator {
            "==" => Operator::Eq,
            "!=" => Operator::Ne,
            ">" => Operator::Gt,
            "<" => Operator::Lt,
            _ => unreachable!(),
        };
        Ok(Self {
            path,
            operator,
            literal,
        })
    }

    pub fn operator(&self) -> &'static str {
        match self.operator {
            Operator::Eq => "==",
            Operator::Ne => "!=",
            Operator::Gt => ">",
            Operator::Lt => "<",
        }
    }

    pub fn evaluate(&self, input: &Value) -> Result<ConditionValue, ConditionError> {
        let mut current = input;
        for segment in &self.path {
            current = current.get(segment).ok_or(ConditionError::UnknownField)?;
        }
        if std::mem::discriminant(current) != std::mem::discriminant(&self.literal) {
            return Err(ConditionError::TypeMismatch);
        }
        let result = match self.operator {
            Operator::Eq => current == &self.literal,
            Operator::Ne => current != &self.literal,
            Operator::Gt | Operator::Lt => {
                let (Some(left), Some(right)) = (current.as_f64(), self.literal.as_f64()) else {
                    return Err(ConditionError::TypeMismatch);
                };
                if matches!(self.operator, Operator::Gt) {
                    left > right
                } else {
                    left < right
                }
            }
        };
        Ok(if result {
            ConditionValue::True
        } else {
            ConditionValue::False
        })
    }
}
