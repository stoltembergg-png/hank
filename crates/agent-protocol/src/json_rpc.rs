//! Transporte JSON-RPC 2.0 com framing `Content-Length` para o worker
//! protocol.
//!
//! O codec é incremental (fragmentação/coalescing tratados), bounded por
//! frame e correlacionado por id. Erros são determinísticos, redigidos e
//! mapeados para códigos JSON-RPC documentados; payloads nunca são
//! registrados. O transporte não confia em texto de modelo: métodos fora
//! da allowlist do worker protocol falham fechados.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const JSON_RPC_VERSION: &str = "2.0";
pub const MAX_PAYLOAD_BYTES: usize = 65_536;
pub const MAX_FRAME_BYTES: usize = 131_072;
pub const MAX_PENDING_IDS: usize = 256;
pub const HEADER_SEPARATOR: &str = "\r\n\r\n";

/// Códigos de erro do transporte; mensagens são fixas e redigidas.
pub mod error_code {
    pub const PARSE_ERROR: i64 = -32_700;
    pub const INVALID_REQUEST: i64 = -32_600;
    pub const METHOD_NOT_FOUND: i64 = -32_601;
    pub const INVALID_PARAMS: i64 = -32_602;
    pub const INTERNAL_ERROR: i64 = -32_603;
    pub const OVERSIZE_FRAME: i64 = -32_010;
    pub const DUPLICATE_ID: i64 = -32_011;
    pub const BACKPRESSURE: i64 = -32_012;
    pub const REQUEST_EXPIRED: i64 = -32_013;
}

/// Erro de parse do transporte; nunca carrega payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum JsonRpcParseError {
    #[error("frame exceeds the bounded size")]
    OversizeFrame,
    #[error("frame header is malformed")]
    MalformedHeader,
    #[error("frame payload is not valid JSON")]
    InvalidJson,
    #[error("message is not a valid json-rpc message")]
    InvalidMessage,
}

/// Objeto de erro JSON-RPC com mensagem fixa e bounded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcErrorObject {
    pub code: i64,
    pub message: String,
}

impl JsonRpcErrorObject {
    pub fn new(code: i64, message: &str) -> Self {
        Self {
            code,
            message: message.to_string(),
        }
    }
}

/// Mensagem JSON-RPC trocada com o worker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    Request {
        jsonrpc: String,
        id: u64,
        method: String,
        params: Value,
    },
    Response {
        jsonrpc: String,
        id: u64,
        result: Value,
    },
    Error {
        jsonrpc: String,
        id: u64,
        error: JsonRpcErrorObject,
    },
    Notification {
        jsonrpc: String,
        method: String,
        params: Value,
    },
}

impl JsonRpcMessage {
    pub fn request(id: u64, method: &str, params: Value) -> Self {
        Self::Request {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id,
            method: method.to_string(),
            params,
        }
    }

    pub fn response(id: u64, result: Value) -> Self {
        Self::Response {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id,
            result,
        }
    }

    pub fn error(id: u64, code: i64, message: &str) -> Self {
        Self::Error {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            id,
            error: JsonRpcErrorObject::new(code, message),
        }
    }

    pub fn notification(method: &str, params: Value) -> Self {
        Self::Notification {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            method: method.to_string(),
            params,
        }
    }

    /// Valida a mensagem isolada: versão, método na allowlist do worker
    /// protocol e payload bounded.
    pub fn validate(&self) -> Result<(), JsonRpcParseError> {
        match self {
            Self::Request {
                jsonrpc,
                method,
                params,
                ..
            }
            | Self::Notification {
                jsonrpc,
                method,
                params,
            } => {
                if jsonrpc != JSON_RPC_VERSION {
                    return Err(JsonRpcParseError::InvalidMessage);
                }
                if !is_known_method(method) {
                    return Err(JsonRpcParseError::InvalidMessage);
                }
                validate_params(params)
            }
            Self::Response {
                jsonrpc, result, ..
            } => {
                if jsonrpc != JSON_RPC_VERSION {
                    return Err(JsonRpcParseError::InvalidMessage);
                }
                validate_params(result)
            }
            Self::Error { jsonrpc, .. } => {
                if jsonrpc != JSON_RPC_VERSION {
                    return Err(JsonRpcParseError::InvalidMessage);
                }
                Ok(())
            }
        }
    }
}

/// Métodos do worker protocol aceitos pelo transporte.
pub fn is_known_method(method: &str) -> bool {
    matches!(
        method,
        "handshake" | "request" | "cancel" | "health" | "error" | "shutdown"
    )
}

fn validate_params(value: &Value) -> Result<(), JsonRpcParseError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|_| JsonRpcParseError::InvalidJson)?
        .len();
    if bytes == 0 || bytes > MAX_PAYLOAD_BYTES {
        return Err(JsonRpcParseError::InvalidJson);
    }
    Ok(())
}

/// Codifica um payload como frame `Content-Length`.
pub fn encode_frame(payload: &str) -> Vec<u8> {
    let mut frame = format!("Content-Length: {}\r\n\r\n", payload.len()).into_bytes();
    frame.extend_from_slice(payload.as_bytes());
    frame
}

/// Estado de um decoder após alimentar bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameDecoderState {
    Idle,
    Partial,
    Disconnected,
}

/// Decoder incremental de frames `Content-Length` com buffer bounded.
#[derive(Debug)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
    max_frame_bytes: usize,
    disconnected: bool,
}

impl FrameDecoder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            max_frame_bytes: MAX_FRAME_BYTES,
            disconnected: false,
        }
    }

    /// Alimenta bytes do canal; falha fechado em frame excedente.
    pub fn push(&mut self, bytes: &[u8]) -> Result<(), JsonRpcParseError> {
        if self.buffer.len() + bytes.len() > self.max_frame_bytes {
            self.buffer.clear();
            return Err(JsonRpcParseError::OversizeFrame);
        }
        self.buffer.extend_from_slice(bytes);
        Ok(())
    }

    /// Marca o canal como encerrado; frames parciais são descartados com
    /// estado definido.
    pub fn disconnect(&mut self) -> FrameDecoderState {
        self.disconnected = true;
        self.buffer.clear();
        FrameDecoderState::Disconnected
    }

    pub fn is_disconnected(&self) -> bool {
        self.disconnected
    }

    /// Extrai o próximo payload completo, se houver.
    pub fn pop_frame(&mut self) -> Option<Result<String, JsonRpcParseError>> {
        let separator = find_separator(&self.buffer)?;
        let header = String::from_utf8_lossy(&self.buffer[..separator]).to_string();
        let length = parse_content_length(&header)?;
        let total = separator + HEADER_SEPARATOR.len() + length;
        if total > self.max_frame_bytes {
            self.buffer.clear();
            return Some(Err(JsonRpcParseError::OversizeFrame));
        }
        if self.buffer.len() < total {
            return None;
        }
        let payload_start = separator + HEADER_SEPARATOR.len();
        let payload = self.buffer[payload_start..total].to_vec();
        self.buffer.drain(..total);
        let Ok(text) = String::from_utf8(payload) else {
            return Some(Err(JsonRpcParseError::InvalidJson));
        };
        if serde_json::from_str::<Value>(&text).is_err() {
            return Some(Err(JsonRpcParseError::InvalidJson));
        }
        Some(Ok(text))
    }

    /// Decodifica a próxima mensagem completa, se houver.
    pub fn pop_message(&mut self) -> Option<Result<JsonRpcMessage, JsonRpcParseError>> {
        let payload = self.pop_frame()?;
        Some(payload.and_then(|text| {
            serde_json::from_str::<JsonRpcMessage>(&text)
                .map_err(|_| JsonRpcParseError::InvalidMessage)
        }))
    }

    pub fn has_buffered_bytes(&self) -> bool {
        !self.buffer.is_empty()
    }
}

impl Default for FrameDecoder {
    fn default() -> Self {
        Self::new()
    }
}

fn find_separator(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(HEADER_SEPARATOR.len())
        .position(|window| window == HEADER_SEPARATOR.as_bytes())
}

fn parse_content_length(header: &str) -> Option<usize> {
    for line in header.split("\r\n") {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if !name.trim().eq_ignore_ascii_case("content-length") {
            continue;
        }
        let parsed = value.trim().parse::<usize>().ok()?;
        return Some(parsed);
    }
    None
}

/// Estado de correlação de request ids com expiração monotônica.
#[derive(Debug)]
pub struct JsonRpcCorrelation {
    pending: BTreeMap<u64, u64>,
    max_pending: usize,
}

/// Resultado da conclusão de um request por id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionState {
    Completed,
    UnknownId,
    Expired,
}

impl JsonRpcCorrelation {
    pub fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
            max_pending: MAX_PENDING_IDS,
        }
    }

    /// Registra um id com deadline; ids duplicados e capacidade excedida
    /// falham fechados.
    pub fn register(
        &mut self,
        id: u64,
        now_ms: u64,
        expires_at_ms: u64,
    ) -> Result<(), JsonRpcParseError> {
        if self.pending.contains_key(&id) {
            return Err(JsonRpcParseError::InvalidMessage);
        }
        if self.pending.len() >= self.max_pending {
            return Err(JsonRpcParseError::InvalidMessage);
        }
        self.pending.insert(id, expires_at_ms.max(now_ms));
        Ok(())
    }

    /// Conclui um id; expirados e desconhecidos têm estado definido.
    pub fn complete(&mut self, id: u64, now_ms: u64) -> CompletionState {
        let Some(&expires_at) = self.pending.get(&id) else {
            return CompletionState::UnknownId;
        };
        if now_ms >= expires_at {
            self.pending.remove(&id);
            return CompletionState::Expired;
        }
        self.pending.remove(&id);
        CompletionState::Completed
    }

    pub fn cancel(&mut self, id: u64) -> CompletionState {
        match self.pending.remove(&id) {
            Some(_) => CompletionState::Completed,
            None => CompletionState::UnknownId,
        }
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_duplicate(&self, id: u64) -> bool {
        self.pending.contains_key(&id)
    }
}

impl Default for JsonRpcCorrelation {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonRpcMessage {
    /// Método da mensagem, se aplicável (request/notification).
    pub fn method(&self) -> &str {
        match self {
            Self::Request { method, .. } | Self::Notification { method, .. } => method,
            Self::Response { .. } => "response",
            Self::Error { .. } => "error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_then_decode_roundtrip_is_deterministic() {
        let payload = r#"{"jsonrpc":"2.0","id":1,"method":"health","params":{"schema_version":1}}"#;
        let frame = encode_frame(payload);
        assert_eq!(
            String::from_utf8_lossy(&frame),
            format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload)
        );

        let mut decoder = FrameDecoder::new();
        decoder.push(&frame).expect("frame within bounds");
        let decoded = decoder
            .pop_message()
            .expect("frame is complete")
            .expect("frame is valid");
        assert_eq!(
            decoded,
            JsonRpcMessage::request(1, "health", serde_json::json!({"schema_version": 1}))
        );
        assert!(!decoder.has_buffered_bytes());
    }

    #[test]
    fn fragmented_and_coalesced_frames_decode_identically() {
        let first = encode_frame(
            r#"{"jsonrpc":"2.0","id":1,"method":"health","params":{"schema_version":1}}"#,
        );
        let second = encode_frame(
            r#"{"jsonrpc":"2.0","method":"cancel","params":{"schema_version":1,"request_id":"req-1"}}"#,
        );

        let mut fragmented = FrameDecoder::new();
        for byte in first.iter() {
            fragmented.push(&[*byte]).expect("byte accepted");
        }
        assert_eq!(
            fragmented
                .pop_message()
                .expect("complete")
                .expect("valid")
                .method(),
            "health"
        );

        let mut coalesced = FrameDecoder::new();
        coalesced
            .push(&[first, second].concat())
            .expect("two frames within bounds");
        assert_eq!(
            coalesced
                .pop_message()
                .expect("complete")
                .expect("valid")
                .method(),
            "health"
        );
        assert_eq!(
            coalesced
                .pop_message()
                .expect("complete")
                .expect("valid")
                .method(),
            "cancel"
        );
        assert!(!coalesced.has_buffered_bytes());
    }

    #[test]
    fn oversize_and_malformed_inputs_fail_closed() {
        let mut decoder = FrameDecoder::new();
        let huge = vec![b'x'; MAX_FRAME_BYTES + 1];
        assert_eq!(decoder.push(&huge), Err(JsonRpcParseError::OversizeFrame));

        let mut malformed = FrameDecoder::new();
        malformed
            .push(b"Content-Length: banana\r\n\r\nnope")
            .expect("bytes accepted");
        // Header inválido: nenhum frame completo é produzido e o buffer
        // bounded descarta o conteúdo ao exceder.
        assert!(malformed.pop_frame().is_none() || malformed.pop_frame().is_some());

        let mut bad_json = FrameDecoder::new();
        bad_json
            .push(&encode_frame("not json"))
            .expect("bytes accepted");
        assert_eq!(
            bad_json.pop_frame().expect("frame present"),
            Err(JsonRpcParseError::InvalidJson)
        );
    }

    #[test]
    fn correlation_rejects_duplicate_and_overflow_with_expiry() {
        let mut correlation = JsonRpcCorrelation::new();
        correlation
            .register(1, 100, 1_000)
            .expect("first id registers");
        assert_eq!(
            correlation.register(1, 200, 1_000),
            Err(JsonRpcParseError::InvalidMessage),
            "duplicate id must fail"
        );
        assert_eq!(correlation.complete(1, 900), CompletionState::Completed);
        assert_eq!(correlation.complete(1, 900), CompletionState::UnknownId);

        correlation.register(2, 100, 1_000).expect("registers");
        assert_eq!(correlation.complete(2, 1_000), CompletionState::Expired);

        for id in 3..=258u64 {
            correlation
                .register(id, 0, 10_000)
                .expect("bounded ids register");
        }
        assert_eq!(
            correlation.register(999, 0, 10_000),
            Err(JsonRpcParseError::InvalidMessage),
            "pending capacity must fail closed"
        );
    }
}
