use std::path::PathBuf;

pub(crate) fn git_program() -> PathBuf {
    let executable = if cfg!(windows) { "git.exe" } else { "git" };
    let path = std::env::var_os("PATH").expect("PATH must be available to locate git");

    std::env::split_paths(&path)
        .map(|directory| directory.join(executable))
        .find(|candidate| candidate.is_file())
        .expect("git executable must be available on PATH")
}
