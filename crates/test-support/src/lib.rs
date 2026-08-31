//! Fixtures de teste e validadores de arquitetura (dev-only).
//!
//! Exporta testes que verificam invariantes arquiteturais:
//! - forbidden-import test: impede que agent-core importe Tauri/SQLx/Tokio/providers
//! - cycle detection: impede ciclos no grafo de dependências do workspace

#[cfg(test)]
pub mod arch_fixtures_test;

pub mod evaluation;
pub mod evaluation_corpus;
pub mod fixtures;
pub mod ids;
