//! Fixtures de teste e validadores de arquitetura (dev-only).
//!
//! Exporta testes que verificam invariantes arquiteturais:
//! - forbidden-import test: impede que agent-core importe Tauri/SQLx/Tokio/providers
//! - cycle detection: impede ciclos no grafo de dependências do workspace

#[cfg(test)]
pub mod arch_fixtures_test;

pub mod agent_loop;
pub mod benchmark_comparison;
mod digest;
pub mod evaluation;
pub mod evaluation_corpus;
pub mod evaluation_runner;
pub mod fixtures;
pub mod fuzz;
pub mod fuzz_targets;
pub mod ids;
pub mod load;
pub mod safety_reasoning_corpus;
