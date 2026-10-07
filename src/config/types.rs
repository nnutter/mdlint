use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(crate) const OPT_IN_RULES: &[&str] = &["MD013", "MD026", "MD033", "MD041", "MD043"];

#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(clippy::struct_excessive_bools)] // config struct mirrors TOML fields 1:1; bools are the right representation
pub struct Config {
    /// Rule configuration: rule name -> config
    #[serde(default)]
    pub rules: HashMap<String, RuleConfig>,

    /// Enable the default rule profile; explicit rule settings override it.
    #[serde(default = "default_default_enabled")]
    pub default_enabled: bool,

    /// Custom rule paths (for future extension)
    #[serde(default)]
    pub custom_rules: Vec<String>,

    /// Respect .gitignore files when discovering files
    #[serde(default = "default_gitignore")]
    pub gitignore: bool,

    /// Front matter pattern (YAML --- or TOML +++)
    #[serde(default)]
    pub front_matter: Option<String>,

    /// Disable inline configuration comments
    #[serde(default)]
    pub no_inline_config: bool,

    /// Paths and glob patterns to exclude from file discovery
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Apply auto-fixes automatically when running `mdlint check`
    #[serde(default = "default_fix")]
    pub fix: bool,
}

fn default_default_enabled() -> bool {
    true
}

fn default_gitignore() -> bool {
    true
}

fn default_fix() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rules: HashMap::new(),
            default_enabled: true,
            custom_rules: Vec::new(),
            gitignore: default_gitignore(),
            front_matter: None,
            no_inline_config: false,
            exclude: Vec::new(),
            fix: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RuleConfig {
    Enabled(bool),
    Config(HashMap<String, toml::Value>),
}

// Legacy field mappings for backward compatibility with old config structure
impl Config {
    /// Legacy accessor for config field (now called rules)
    #[must_use]
    pub fn config(&self) -> &HashMap<String, RuleConfig> {
        &self.rules
    }

    /// Apply `--select`/`--ignore` CLI overrides on top of the loaded config.
    ///
    /// `select` (if non-empty and not `ALL`, case-insensitive) restricts linting to just the
    /// listed rules: `default_enabled` is turned off and each rule gets a bare `Enabled(true)`
    /// entry, unless a more specific `RuleConfig::Config(..)` already exists for it (so
    /// config-file rule parameters, e.g. MD013's `line_length`, survive `--select`).
    ///
    /// `ignore` unconditionally force-disables the listed rules, overriding both the config
    /// file and `--select`.
    ///
    /// `ALL` enables the default profile and opts into optional rules, retaining
    /// explicit rule settings. Empty `select`/`ignore` is a no-op.
    #[must_use]
    pub fn apply_rule_filters(mut self, select: &[String], ignore: &[String]) -> Self {
        let select_all = select.iter().any(|code| code.eq_ignore_ascii_case("all"));
        if select_all {
            self.default_enabled = true;
            for rule in OPT_IN_RULES {
                self.rules
                    .entry((*rule).to_owned())
                    .or_insert(RuleConfig::Enabled(true));
            }
        } else if !select.is_empty() {
            self.default_enabled = false;
            for code in select {
                self.rules
                    .entry(code.to_uppercase())
                    .or_insert(RuleConfig::Enabled(true));
            }
        }

        for code in ignore {
            self.rules
                .insert(code.to_uppercase(), RuleConfig::Enabled(false));
        }

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_empty_is_noop() {
        let config = Config::default().apply_rule_filters(&[], &[]);
        assert!(config.default_enabled);
        assert!(config.rules.is_empty());
    }

    #[test]
    fn select_restricts_to_listed_rules() {
        let config = Config::default().apply_rule_filters(&["md001".to_owned()], &[]);
        assert!(!config.default_enabled);
        assert!(matches!(
            config.rules.get("MD001"),
            Some(RuleConfig::Enabled(true))
        ));
        assert_eq!(config.rules.len(), 1);
    }

    #[test]
    fn select_all_enables_optional_checks() {
        let config = Config::default().apply_rule_filters(&["ALL".to_owned()], &[]);
        let content = format!("{}\n", "word ".repeat(30));
        let violations = crate::lint::LintEngine::new(config)
            .lint_content(&content)
            .unwrap();
        assert!(violations.iter().any(|v| v.rule == "MD013"));
    }

    #[test]
    fn select_preserves_existing_rule_config() {
        let mut base = Config::default();
        let mut params = HashMap::new();
        params.insert("line_length".to_owned(), toml::Value::Integer(100));
        base.rules
            .insert("MD013".to_owned(), RuleConfig::Config(params));

        let config = base.apply_rule_filters(&["MD013".to_owned()], &[]);
        match config.rules.get("MD013") {
            Some(RuleConfig::Config(params)) => {
                assert_eq!(params.get("line_length"), Some(&toml::Value::Integer(100)));
            }
            other => panic!("expected preserved MD013 config, got {other:?}"),
        }
    }

    #[test]
    fn ignore_force_disables_rule() {
        let mut base = Config::default();
        base.rules
            .insert("MD013".to_owned(), RuleConfig::Enabled(true));

        let config = base.apply_rule_filters(&[], &["md013".to_owned()]);
        assert!(matches!(
            config.rules.get("MD013"),
            Some(RuleConfig::Enabled(false))
        ));
    }

    #[test]
    fn ignore_wins_over_select() {
        let config = Config::default().apply_rule_filters(
            &["MD001".to_owned(), "MD013".to_owned()],
            &["MD013".to_owned()],
        );
        assert!(matches!(
            config.rules.get("MD001"),
            Some(RuleConfig::Enabled(true))
        ));
        assert!(matches!(
            config.rules.get("MD013"),
            Some(RuleConfig::Enabled(false))
        ));
    }
}
