//! Resolução do diretório de dados do NEXUS.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Variável de ambiente que sobrescreve o diretório de dados.
pub const DATA_DIR_ENV: &str = "NEXUS_DATA_DIR";
const APP_DIR_NAME: &str = "Nexus";

/// Caminhos de dados locais. Todos derivam de `root`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataPaths {
    root: PathBuf,
}

impl DataPaths {
    /// Usa `NEXUS_DATA_DIR` quando definido; caso contrário o diretório de dados local da plataforma
    /// (`%LOCALAPPDATA%\Nexus` no Windows).
    pub fn resolve() -> Result<Self> {
        Self::resolve_from(std::env::var_os(DATA_DIR_ENV).map(PathBuf::from))
    }

    fn resolve_from(override_dir: Option<PathBuf>) -> Result<Self> {
        if let Some(dir) = override_dir.filter(|d| !d.as_os_str().is_empty()) {
            if !dir.is_absolute() {
                return Err(Error::InvalidInput(format!(
                    "{DATA_DIR_ENV} precisa ser um caminho absoluto (recebido: {})",
                    dir.display()
                )));
            }
            return Ok(Self::at(dir));
        }
        let base = dirs::data_local_dir().ok_or_else(|| {
            Error::Internal("não foi possível determinar o diretório de dados do usuário".into())
        })?;
        Ok(Self::at(base.join(APP_DIR_NAME)))
    }

    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn db(&self) -> PathBuf {
        self.root.join("nexus.db")
    }

    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn backups(&self) -> PathBuf {
        self.root.join("backups")
    }

    /// Cria os diretórios necessários.
    pub fn ensure(&self) -> Result<()> {
        for dir in [self.root.clone(), self.logs(), self.backups()] {
            std::fs::create_dir_all(&dir)
                .map_err(|e| Error::io(format!("criar {}", dir.display()), e))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_must_be_absolute() {
        let err = DataPaths::resolve_from(Some(PathBuf::from("relativo/dir"))).unwrap_err();
        assert_eq!(err.code(), crate::ErrorCode::InvalidInput);
    }

    #[test]
    fn override_is_used_verbatim() {
        let dir = std::env::temp_dir().join("nexus-paths-test");
        let paths = DataPaths::resolve_from(Some(dir.clone())).unwrap();
        assert_eq!(paths.root(), dir);
        assert_eq!(paths.db(), dir.join("nexus.db"));
    }

    #[test]
    fn empty_override_falls_back_to_platform_dir() {
        let paths = DataPaths::resolve_from(Some(PathBuf::new())).unwrap();
        assert!(paths.root().ends_with(APP_DIR_NAME));
    }
}
