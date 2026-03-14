use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct SpaceConfig {
    pub name: String,
    pub host: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub default_space: String,
    #[serde(default)]
    pub spaces: Vec<SpaceConfig>,
}

pub fn config_path() -> PathBuf {
    // Respect XDG_CONFIG_HOME on Linux; fall back to ~/.config on macOS
    let config_home = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".config")
        });
    config_home.join("lazybacklog").join("config.toml")
}

pub fn load() -> Result<Config> {
    let path = config_path();
    if !path.exists() {
        bail!(
            "Config file not found: {}\nCreate it with your Backlog spaces and API keys.",
            path.display()
        );
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config: {}", path.display()))?;
    let config: Config =
        toml::from_str(&content).with_context(|| "Failed to parse config.toml")?;
    if config.spaces.is_empty() {
        bail!("No spaces defined in config.toml");
    }
    if !config.spaces.iter().any(|s| s.name == config.default_space) {
        bail!(
            "default_space '{}' not found in spaces",
            config.default_space
        );
    }
    Ok(config)
}

/// Returns a warning string if permissions are wrong, None if OK.
#[cfg(unix)]
pub fn check_permissions(path: &std::path::Path) -> Option<String> {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = std::fs::metadata(path) {
        let mode = metadata.permissions().mode() & 0o777;
        if mode != 0o600 {
            return Some(format!(
                "Warning: {} has permissions {:04o}, expected 0600. Run: chmod 600 {}",
                path.display(),
                mode,
                path.display()
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_config(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    fn load_from_path(path: &std::path::Path) -> Result<Config> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read: {}", path.display()))?;
        let config: Config =
            toml::from_str(&content).with_context(|| "Failed to parse config.toml")?;
        if config.spaces.is_empty() {
            bail!("No spaces defined in config.toml");
        }
        if !config.spaces.iter().any(|s| s.name == config.default_space) {
            bail!(
                "default_space '{}' not found in spaces",
                config.default_space
            );
        }
        Ok(config)
    }

    #[test]
    fn test_parse_valid_config() {
        let file = write_config(
            r#"
default_space = "myspace"

[[spaces]]
name = "myspace"
host = "myspace.backlog.com"
api_key = "abc123"
"#,
        );
        let config = load_from_path(file.path()).unwrap();
        assert_eq!(config.default_space, "myspace");
        assert_eq!(config.spaces.len(), 1);
        assert_eq!(config.spaces[0].host, "myspace.backlog.com");
        assert_eq!(config.spaces[0].api_key, "abc123");
    }

    #[test]
    fn test_parse_multiple_spaces() {
        let file = write_config(
            r#"
default_space = "work"

[[spaces]]
name = "myspace"
host = "myspace.backlog.com"
api_key = "abc123"

[[spaces]]
name = "work"
host = "work.backlog.jp"
api_key = "def456"
"#,
        );
        let config = load_from_path(file.path()).unwrap();
        assert_eq!(config.spaces.len(), 2);
        assert_eq!(config.default_space, "work");
    }

    #[test]
    fn test_invalid_default_space() {
        let file = write_config(
            r#"
default_space = "nonexistent"

[[spaces]]
name = "myspace"
host = "myspace.backlog.com"
api_key = "abc123"
"#,
        );
        let err = load_from_path(file.path()).unwrap_err();
        assert!(err.to_string().contains("default_space"));
    }

    #[test]
    fn test_empty_spaces() {
        let file = write_config(r#"default_space = "x""#);
        let err = load_from_path(file.path()).unwrap_err();
        assert!(err.to_string().contains("No spaces"));
    }

    #[cfg(unix)]
    #[test]
    fn test_permissions_ok() {
        use std::os::unix::fs::PermissionsExt;
        let file = write_config("x = 1");
        std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(check_permissions(file.path()).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn test_permissions_warn() {
        use std::os::unix::fs::PermissionsExt;
        let file = write_config("x = 1");
        std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o644)).unwrap();
        let warning = check_permissions(file.path());
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("chmod 600"));
    }
}
