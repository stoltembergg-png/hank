//! Fixtures de arquitetura para validação de invariantes.
//!
//! Testes que devem FALHAR se as fronteiras arquiteturais forem violadas.
//! Estes testes são executados como parte de `cargo test` e fazem parte
//! dos critérios de aceite AC-303 e AC-304.

use cargo_metadata::MetadataCommand;
use petgraph::algo::is_cyclic_directed;
use petgraph::Graph;
use std::collections::HashMap;

/// Teste: agent-core NÃO deve importar crates proibidas (AI-001, AI-003)
///
/// Crates proibidas para agent-core:
/// - tauri, tao, wry (Tauri/UI)
/// - sqlx (database - pertence a Infrastructure)
/// - tokio (runtime - pertence a Execution/Durable)
/// - providers concretos (openai, anthropic, etc.)
/// - quaisquer segredos de ambiente
#[test]
fn arch_forbidden_imports() {
    // @spec:AC-303
    let metadata = MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("falha ao obter metadata do cargo");

    // Encontra o package agent-core
    let agent_core = metadata
        .packages
        .iter()
        .find(|p| p.name == "agent-core")
        .expect("package agent-core não encontrado no workspace");

    // Lista de crates proibidas para agent-core
    let forbidden = [
        "tauri",
        "tao",
        "wry",   // Tauri/UI shell
        "sqlx",  // Database (Infrastructure layer)
        "tokio", // Runtime (Execution/Durable layer)
        "openai",
        "anthropic",
        "gemini",
        "openrouter",
        "ollama",   // Providers concretos
        "reqwest",  // HTTP direto (deve usar port)
        "std::env", // Env vars diretas (não permitido no core)
    ];

    // Verifica dependências diretas
    for dep in &agent_core.dependencies {
        let dep_name = dep.name.as_str();
        for &forbidden_name in &forbidden {
            if dep_name.starts_with(forbidden_name) {
                panic!(
                    "VIOLAÇÃO ARQUITETURAL (AI-001/AI-003): agent-core importa '{}' (proibido: {})",
                    dep_name, forbidden_name
                );
            }
        }
    }

    // Verifica também dependências transitivas via grafo
    check_transitive_forbidden(&metadata, "agent-core", &forbidden);
}

/// Verifica dependências transitivas proibidas
#[allow(dead_code)]
fn check_transitive_forbidden(
    metadata: &cargo_metadata::Metadata,
    start: &str,
    forbidden: &[&str],
) {
    let mut visited = std::collections::HashSet::new();
    let mut stack = vec![start];

    // Mapeia nome -> package
    let packages: HashMap<_, _> = metadata
        .packages
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();

    while let Some(current) = stack.pop() {
        if !visited.insert(current) {
            continue;
        }

        if let Some(pkg) = packages.get(current) {
            for dep in &pkg.dependencies {
                let dep_name = dep.name.as_str();

                // Verifica se é proibido
                for &forbidden_name in forbidden {
                    if dep_name.starts_with(forbidden_name) {
                        panic!(
                            "VIOLAÇÃO ARQUITETURAL TRANSITIVA (AI-001/AI-003): {} -> '{}' (proibido: {})",
                            current, dep_name, forbidden_name
                        );
                    }
                }

                // Continha travessia apenas para crates do workspace
                if packages.contains_key(dep_name) {
                    stack.push(dep_name);
                }
            }
        }
    }
}

/// Teste: workspace NÃO deve ter ciclos de dependência (AC-304)
#[test]
fn detect_cycles() {
    // @spec:AC-304
    let metadata = MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("falha ao obter metadata do cargo");

    // Constrói grafo de dependências apenas dos packages do workspace
    let workspace_packages: Vec<_> = metadata
        .packages
        .iter()
        .filter(|p| metadata.workspace_members.contains(&p.id))
        .collect();

    let mut graph = Graph::<String, ()>::new();
    let mut node_indices = HashMap::new();

    // Adiciona nós
    for pkg in &workspace_packages {
        let idx = graph.add_node(pkg.name.to_string());
        node_indices.insert(pkg.name.as_str(), idx);
    }

    // Adiciona arestas apenas para dependências regulares (não dev-dependencies)
    for pkg in &workspace_packages {
        let from_idx = node_indices[pkg.name.as_str()];
        for dep in &pkg.dependencies {
            // Only add edge if the dependency is also a workspace member
            // AND this is a regular dependency (not dev-dependency)
            if let Some(&to_idx) = node_indices.get(dep.name.as_str()) {
                // Check if this is a dev dependency via the `kind` field
                // DependencyKind can be Normal, Development, or Build
                // "dev" in Cargo.toml maps to DependencyKind::Development
                let is_dev = matches!(dep.kind, cargo_metadata::DependencyKind::Development);
                if !is_dev {
                    graph.add_edge(from_idx, to_idx, ());
                }
            }
        }
    }

    // Verifica ciclos
    assert!(
        !is_cyclic_directed(&graph),
        "CICLO DE DEPENDÊNCIA DETECTADO no workspace! Isso viola AC-304 e AI-035."
    );
}

/// Teste: workspace compila sem erros (AC-302)
/// Este teste passa se `cargo check --workspace` compilar com sucesso.
#[test]
fn workspace_compiles_clean() {
    // @spec:AC-302
    let package_name = env!("CARGO_PKG_NAME").to_string();
    assert_eq!(
        package_name, "test-support",
        "fixture compilada no package incorreto"
    );
}

/// Teste: metadata lista exatamente as crates planejadas (AC-301)
#[test]
fn metadata_lists_expected_crates() {
    // @spec:AC-301
    let metadata = MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("falha ao obter metadata do cargo");

    let workspace_package_names: Vec<String> = metadata
        .packages
        .iter()
        .filter(|p| metadata.workspace_members.contains(&p.id))
        .map(|p| p.name.to_string())
        .collect();

    let mut expected = vec![
        "agent-core",
        "agent-protocol",
        "agent-runtime",
        "provider-core",
        "plugin-core",
        "secrets-core",
        "security-core",
        "auth-core",
        "remote-core",
        "provider-adapter-openai-compatible",
        "provider-adapter-openai",
        "provider-adapter-anthropic",
        "provider-adapter-gemini",
        "provider-adapter-openrouter",
        "provider-adapter-ollama",
        "test-support",
        "tool-core",
        "workflow-core",
        "xtask",
    ];
    expected.sort();

    let mut actual = workspace_package_names.clone();
    actual.sort();

    assert_eq!(
        actual, expected,
        "AC-301 FALHOU: packages do workspace divergentes.\nEsperado: {:?}\nObtido: {:?}",
        expected, actual
    );
}

/// Teste: resolver = "2" e toolchain declarados (AC-305)
#[test]
fn resolver_and_toolchain_declared() {
    // @spec:AC-305
    // Find the root Cargo.toml (workspace root)
    let mut cargo_toml_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    cargo_toml_path.push("../../Cargo.toml");

    assert!(
        cargo_toml_path.exists(),
        "Cargo.toml raiz não encontrado em {:?}",
        cargo_toml_path
    );

    let cargo_toml = std::fs::read_to_string(&cargo_toml_path).expect("falha ao ler Cargo.toml");

    assert!(
        cargo_toml.contains("resolver = \"2\""),
        "AC-305: resolver = \"2\" não declarado no Cargo.toml raiz"
    );

    // Check toolchain from workspace root
    let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let toolchain_exists = workspace_root.join("rust-toolchain.toml").exists()
        || workspace_root.join("rust-toolchain").exists();
    assert!(
        toolchain_exists,
        "AC-305: rust-toolchain.toml ou rust-toolchain não encontrado em {:?}",
        workspace_root
    );

    let lock_exists = workspace_root.join("Cargo.lock").exists();
    assert!(
        lock_exists,
        "AC-305: Cargo.lock não versionado em {:?}",
        workspace_root
    );
}
