//! Configuration loader
//!
//! Handles loading configuration from YAML files with environment variable expansion.

use std::path::Path;
use std::sync::{Arc, RwLock};

use crate::error::{HarnessError, Result};
use crate::error::kinds::ConfigError as ConfigErrorKind;

use super::app::AppConfig;

/// Global configuration cache
///
/// Uses Arc<RwLock> for thread-safe shared access.
static CONFIG_CACHE: RwLock<Option<Arc<AppConfig>>> = RwLock::new(None);

/// Load configuration from a YAML file
///
/// This function loads the configuration from the specified path,
/// performs environment variable substitution, and caches the result.
///
/// # Arguments
///
/// * `path` - Path to the YAML configuration file
///
/// # Returns
///
/// Returns the loaded `AppConfig` or a `ConfigError`.
///
/// # Examples
///
/// ```no_run
/// use agent_harness::config::load_config;
///
/// # fn main() -> agent_harness::Result<()> {
/// let config = load_config("config.yaml")?;
/// # Ok(())
/// # }
/// ```
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Arc<AppConfig>> {
    let path = path.as_ref();

    // Check if we have a cached config
    {
        let cache = CONFIG_CACHE.read().map_err(|e| {
            HarnessError::Config(ConfigErrorKind::LoadError(
                path.display().to_string(),
                format!("Cache lock error: {}", e),
            ))
        })?;

        if let Some(ref config) = *cache {
            return Ok(Arc::clone(config));
        }
    }

    // Load the configuration file
    let content = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            HarnessError::Config(ConfigErrorKind::FileNotFound(path.display().to_string()))
        } else {
            HarnessError::Config(ConfigErrorKind::LoadError(
                path.display().to_string(),
                format!("Read error: {}", e),
            ))
        }
    })?;

    // Expand environment variables
    let expanded = expand_env_vars(&content)?;

    // Parse YAML
    let config: AppConfig = serde_yaml::from_str(&expanded).map_err(|e| {
        HarnessError::Config(ConfigErrorKind::LoadError(
            path.display().to_string(),
            format!("Parse error: {}", e),
        ))
    })?;

    // Cache the config
    let config = Arc::new(config);
    {
        let mut cache = CONFIG_CACHE.write().map_err(|e| {
            HarnessError::Config(ConfigErrorKind::LoadError(
                path.display().to_string(),
                format!("Cache lock error: {}", e),
            ))
        })?;
        *cache = Some(Arc::clone(&config));
    }

    Ok(config)
}

/// Clear the configuration cache
///
/// This forces the next call to `load_config` to reload from disk.
pub fn clear_config_cache() {
    if let Ok(mut cache) = CONFIG_CACHE.write() {
        *cache = None;
    }
}

/// Expand environment variables in a string
///
/// Replaces patterns like `$VAR_NAME` or `${VAR_NAME}` with their
/// corresponding environment variable values.
///
/// Uses regex-based matching for more robust variable name extraction.
///
/// # Arguments
///
/// * `input` - The input string with potential environment variable references
///
/// # Returns
///
/// The string with environment variables expanded
fn expand_env_vars(input: &str) -> Result<String> {
    use regex::Regex;

    // Match ${VAR_NAME} first (braced form)
    let re_braced = Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}").map_err(|e| {
        HarnessError::other(format!("Regex compilation error: {}", e))
    })?;

    let result = re_braced.replace_all(input, |caps: &regex::Captures| {
        let var_name = &caps[1];
        std::env::var(var_name).unwrap_or_else(|_| format!("${{{}}}", var_name))
    });

    // Match $VAR_NAME (unbraced form)
    // This regex matches variable names but tries shorter prefixes if not found
    let mut unbraced_result = String::new();
    let mut pos = 0;
    while pos < result.len() {
        if let Some(start) = result[pos..].find('$') {
            let start = pos + start;
            unbraced_result.push_str(&result[pos..start]);

            // Try to extract a variable name starting after '$'
            let remaining = &result[start + 1..];
            let mut var_end = 0;
            for c in remaining.chars() {
                if c.is_alphanumeric() || c == '_' {
                    var_end += c.len_utf8();
                } else {
                    break;
                }
            }

            if var_end > 0 {
                let var_name = &remaining[..var_end];

                // Try to find the variable, trying shorter prefixes if needed
                let mut found_value = None;
                for i in (1..=var_name.len()).rev() {
                    let prefix = &var_name[..i];
                    if let Ok(value) = std::env::var(prefix) {
                        found_value = Some((prefix.len(), value));
                        break;
                    }
                }

                if let Some((len, value)) = found_value {
                    unbraced_result.push_str(&value);
                    pos = start + 1 + len;
                } else {
                    // Variable not found, keep the $ and try the next character
                    unbraced_result.push('$');
                    pos = start + 1;
                }
            } else {
                // '$' at end of string or followed by non-variable character
                unbraced_result.push('$');
                pos = start + 1;
            }
        } else {
            unbraced_result.push_str(&result[pos..]);
            break;
        }
    }

    Ok(unbraced_result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Generate unique test IDs to avoid conflicts between parallel tests
    fn test_var_name(suffix: &str) -> String {
        format!(
            "HARNESS_TEST_{}_{}",
            std::thread::current()
                .name()
                .unwrap_or("unknown")
                .replace("::", "_")
                .replace(" ", "_"),
            suffix
        )
    }

    #[test]
    fn test_expand_env_vars_simple() {
        let var_name = test_var_name("SIMPLE");
        std::env::set_var(&var_name, "hello");
        let input = format!("prefix_{}{}_suffix", "$", var_name);
        let result = expand_env_vars(&input).unwrap();
        assert_eq!(result, "prefix_hello_suffix");
        std::env::remove_var(&var_name);
    }

    #[test]
    fn test_expand_env_vars_braced() {
        let var_name = test_var_name("BRACED");
        std::env::set_var(&var_name, "world");
        // Construct "prefix_${VAR_NAME}_suffix" manually
        let mut input = String::from("prefix_${");
        input.push_str(&var_name);
        input.push_str("}_suffix");
        let result = expand_env_vars(&input).unwrap();
        assert_eq!(result, "prefix_world_suffix");
        std::env::remove_var(&var_name);
    }

    #[test]
    fn test_expand_env_vars_missing() {
        let input = "prefix_$HARNESS_MISSING_VAR_suffix";
        let result = expand_env_vars(input).unwrap();
        assert_eq!(result, "prefix_$HARNESS_MISSING_VAR_suffix");
    }

    #[test]
    fn test_expand_env_vars_multiple() {
        let var1 = test_var_name("VAR1");
        let var2 = test_var_name("VAR2");
        std::env::set_var(&var1, "a");
        std::env::set_var(&var2, "b");
        // Construct "${VAR1}-${VAR2}-${VAR1}" manually
        let mut input = String::new();
        input.push_str("${");
        input.push_str(&var1);
        input.push_str("}-${");
        input.push_str(&var2);
        input.push_str("}-${");
        input.push_str(&var1);
        input.push('}');
        let result = expand_env_vars(&input).unwrap();
        assert_eq!(result, "a-b-a");
        std::env::remove_var(&var1);
        std::env::remove_var(&var2);
    }

    #[test]
    fn test_clear_cache() {
        clear_config_cache();
        // If this compiles and runs without panic, the function works
        // (actual cache testing would require a file on disk)
    }
}
