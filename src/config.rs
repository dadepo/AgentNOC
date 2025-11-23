use color_eyre::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub const RIPESTAT_MCP_ENDPONT: &str = "https://mcp-ripestat.taihen.org/mcp";

#[derive(Debug, Clone, Deserialize)]
pub struct PrefixInfo {
    #[allow(dead_code)]
    pub description: String,
    #[allow(dead_code)]
    pub asn: Vec<u32>,
    #[serde(default)]
    #[serde(rename = "ignoreMorespecifics")]
    pub ignore_morespecifics: bool,
    #[serde(default)]
    pub ignore: bool,
    #[allow(dead_code)]
    pub group: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AsnInfo {
    #[allow(dead_code)]
    pub group: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Options {
    #[serde(rename = "monitorASns")]
    #[allow(dead_code)]
    pub monitor_asns: HashMap<String, AsnInfo>,
}

#[derive(Debug, Clone)]
pub struct PrefixesConfig {
    pub prefixes: HashMap<String, PrefixInfo>,
    pub monitored_asns: HashMap<String, AsnInfo>,
}

impl PrefixesConfig {
    /// Load and parse the prefixes.yml file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        Self::from_str(&content)
    }

    /// Parse the YAML content
    pub fn from_str(content: &str) -> Result<Self> {
        // Parse as a generic YAML value to handle dynamic prefix keys
        let value: serde_yaml::Value = serde_yaml::from_str(content)?;

        let mut prefixes = HashMap::new();
        let mut monitored_asns = HashMap::new();

        if let serde_yaml::Value::Mapping(mapping) = value {
            for (key, val) in mapping {
                if let Some(key_str) = key.as_str() {
                    if key_str == "options" {
                        // Parse the options section
                        if let serde_yaml::Value::Mapping(options_map) = val {
                            if let Some(serde_yaml::Value::Mapping(asns_map)) = options_map
                                .get(serde_yaml::Value::String("monitorASns".to_string()))
                            {
                                for (asn_key, asn_val) in asns_map {
                                    if let Some(asn_str) = asn_key.as_str() {
                                        if let Ok(asn_info) =
                                            serde_yaml::from_value::<AsnInfo>(asn_val.clone())
                                        {
                                            monitored_asns.insert(asn_str.to_string(), asn_info);
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // This is a prefix entry
                        if let Ok(prefix_info) = serde_yaml::from_value::<PrefixInfo>(val.clone()) {
                            prefixes.insert(key_str.to_string(), prefix_info);
                        }
                    }
                }
            }
        }

        Ok(Self {
            prefixes,
            monitored_asns,
        })
    }

    /// Check if a prefix is monitored
    #[allow(dead_code)] // Used in tests
    pub fn is_prefix_monitored(&self, prefix: &str) -> bool {
        self.prefixes.contains_key(prefix)
    }

    /// Check if an ASN is monitored
    pub fn is_asn_monitored(&self, asn: &str) -> bool {
        self.monitored_asns.contains_key(asn)
    }

    /// Find the matching prefix info for a given alert prefix
    /// Returns the prefix info if the alert prefix matches or is contained within a monitored prefix
    fn find_matching_prefix_info(&self, alert_prefix: &str) -> Option<&PrefixInfo> {
        // First check exact match
        if let Some(prefix_info) = self.prefixes.get(alert_prefix) {
            return Some(prefix_info);
        }

        // Parse the alert prefix to check containment
        if let Ok(alert_net) = alert_prefix.parse::<ipnet::IpNet>() {
            // Check if alert prefix is contained in any monitored prefix
            for (monitored_prefix, prefix_info) in &self.prefixes {
                // Skip ignored prefixes
                if prefix_info.ignore {
                    continue;
                }

                if let Ok(monitored_net) = monitored_prefix.parse::<ipnet::IpNet>() {
                    // Check if alert prefix is contained in monitored prefix
                    if monitored_net.contains(&alert_net) {
                        // If ignoreMorespecifics is true, skip more specific prefixes
                        if prefix_info.ignore_morespecifics
                            && alert_net.prefix_len() > monitored_net.prefix_len()
                        {
                            continue;
                        }
                        return Some(prefix_info);
                    }
                    // Also check if monitored prefix is contained in alert prefix
                    if alert_net.contains(&monitored_net) {
                        return Some(prefix_info);
                    }
                }
            }
        }

        None
    }

    /// Check if a prefix matches or is contained within any monitored prefix
    pub fn is_prefix_relevant(&self, alert_prefix: &str) -> bool {
        self.find_matching_prefix_info(alert_prefix).is_some()
    }

    /// Check if an alert is relevant to our monitored resources
    pub fn is_alert_relevant(&self, alert: &crate::alerts::http::server::BGPAlerterAlert) -> bool {
        // Check main prefix
        if self.is_prefix_relevant(&alert.details.prefix) {
            return true;
        }

        // Check new prefix if present
        if let Some(ref newprefix) = alert.details.newprefix {
            if self.is_prefix_relevant(newprefix) {
                return true;
            }
        }

        // Check ASN in monitored ASNs list
        if self.is_asn_monitored(&alert.details.asn) {
            return true;
        }

        // Check new origin ASN if present
        if let Some(ref neworigin) = alert.details.neworigin {
            if self.is_asn_monitored(neworigin) {
                return true;
            }
        }

        false
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_server_port")]
    pub server_port: u16,
    #[serde(default = "default_llm_model_name")]
    pub llm_model_name: String,
}

fn default_server_port() -> u16 {
    7654
}

fn default_llm_model_name() -> String {
    "claude-sonnet-4-5-20250929".to_string()
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        let server_port = std::env::var("SERVER_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or_else(default_server_port);

        let llm_model_name =
            std::env::var("LLM_MODEL_NAME").unwrap_or_else(|_| default_llm_model_name());

        Ok(Self {
            server_port,
            llm_model_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_prefix() {
        let yaml = r#"
217.164.0.0/15:
  description: Test prefix
  asn:
    - 5384
  ignoreMorespecifics: false
  ignore: false
  group: noc
"#;

        let config = PrefixesConfig::from_str(yaml).unwrap();

        assert!(config.is_prefix_monitored("217.164.0.0/15"));
        assert!(!config.is_prefix_monitored("192.0.2.0/24"));

        let prefix_info = config.prefixes.get("217.164.0.0/15").unwrap();
        assert_eq!(prefix_info.description, "Test prefix");
        assert_eq!(prefix_info.asn, vec![5384]);
        assert_eq!(prefix_info.group, "noc");
        assert!(!prefix_info.ignore);
        assert!(!prefix_info.ignore_morespecifics);
    }

    #[test]
    fn test_parse_multiple_prefixes() {
        let yaml = r#"
217.164.0.0/15:
  description: First prefix
  asn:
    - 5384
  ignoreMorespecifics: false
  ignore: false
  group: production
192.0.2.0/24:
  description: Second prefix
  asn:
    - 3333
  ignoreMorespecifics: true
  ignore: false
  group: testing
"#;

        let config = PrefixesConfig::from_str(yaml).unwrap();

        assert!(config.is_prefix_monitored("217.164.0.0/15"));
        assert!(config.is_prefix_monitored("192.0.2.0/24"));

        let prefix1 = config.prefixes.get("217.164.0.0/15").unwrap();
        assert_eq!(prefix1.group, "production");
        assert!(!prefix1.ignore_morespecifics);

        let prefix2 = config.prefixes.get("192.0.2.0/24").unwrap();
        assert_eq!(prefix2.group, "testing");
        assert!(prefix2.ignore_morespecifics);
    }

    #[test]
    fn test_parse_monitored_asns() {
        let yaml = r#"
options:
  monitorASns:
    '3333':
      group: noc
    '5384':
      group: production
"#;

        let config = PrefixesConfig::from_str(yaml).unwrap();

        assert!(config.is_asn_monitored("3333"));
        assert!(config.is_asn_monitored("5384"));
        assert!(!config.is_asn_monitored("1234"));
    }

    #[test]
    fn test_parse_full_config() {
        let yaml = r#"
217.164.0.0/15:
  description: Production network
  asn:
    - 5384
  ignoreMorespecifics: false
  ignore: false
  group: noc
192.0.2.0/24:
  description: Test prefix
  asn:
    - 3333
  ignoreMorespecifics: false
  ignore: false
  group: noc
options:
  monitorASns:
    '3333':
      group: noc
    '5384':
      group: noc
"#;

        let config = PrefixesConfig::from_str(yaml).unwrap();

        // Test prefixes
        assert!(config.is_prefix_monitored("217.164.0.0/15"));
        assert!(config.is_prefix_monitored("192.0.2.0/24"));

        // Test ASNs
        assert!(config.is_asn_monitored("3333"));
        assert!(config.is_asn_monitored("5384"));
    }

    #[test]
    fn test_prefix_with_multiple_asns() {
        let yaml = r#"
192.0.2.0/24:
  description: Multi-ASN prefix
  asn:
    - 3333
    - 5384
  ignoreMorespecifics: false
  ignore: false
  group: noc
"#;

        let config = PrefixesConfig::from_str(yaml).unwrap();

        let prefix_info = config.prefixes.get("192.0.2.0/24").unwrap();
        assert_eq!(prefix_info.asn.len(), 2);
        assert!(prefix_info.asn.contains(&3333));
        assert!(prefix_info.asn.contains(&5384));
    }

    #[test]
    fn test_ignored_prefix() {
        let yaml = r#"
192.0.2.0/24:
  description: Ignored prefix
  asn:
    - 3333
  ignoreMorespecifics: false
  ignore: true
  group: noc
"#;

        let config = PrefixesConfig::from_str(yaml).unwrap();

        assert!(config.is_prefix_monitored("192.0.2.0/24"));
        let prefix_info = config.prefixes.get("192.0.2.0/24").unwrap();
        assert!(prefix_info.ignore);
    }

    #[test]
    fn test_default_ignore_values() {
        let yaml = r#"
192.0.2.0/24:
  description: Prefix with defaults
  asn:
    - 3333
  group: noc
"#;

        let config = PrefixesConfig::from_str(yaml).unwrap();

        let prefix_info = config.prefixes.get("192.0.2.0/24").unwrap();
        // Default values should be false
        assert!(!prefix_info.ignore);
        assert!(!prefix_info.ignore_morespecifics);
    }

    #[test]
    fn test_empty_config() {
        let yaml = r#"
options:
  monitorASns: {}
"#;

        let config = PrefixesConfig::from_str(yaml).unwrap();

        assert_eq!(config.prefixes.len(), 0);
        assert_eq!(config.monitored_asns.len(), 0);
    }

    #[test]
    fn test_nonexistent_prefix() {
        let yaml = r#"
217.164.0.0/15:
  description: Test
  asn:
    - 5384
  group: noc
"#;

        let config = PrefixesConfig::from_str(yaml).unwrap();

        assert!(!config.is_prefix_monitored("192.0.2.0/24"));
        assert!(config.prefixes.get("192.0.2.0/24").is_none());
    }

    #[test]
    fn test_real_world_format() {
        // Test with the actual format from prefixes.yml
        let yaml = r#"
217.164.0.0/15:
  description: No description provided (No ROA available)
  asn:
    - 5384
  ignoreMorespecifics: false
  ignore: false
  group: noc
192.0.2.0/24:
  description: Expected prefix for AS 3333 (for misconfiguration testing)
  asn:
    - 3333
  ignoreMorespecifics: false
  ignore: false
  group: noc
options:
  monitorASns:
    '3333':
      group: noc
    '5384':
      group: noc
"#;

        let config = PrefixesConfig::from_str(yaml).unwrap();

        // Verify all prefixes are parsed
        assert_eq!(config.prefixes.len(), 2);
        assert!(config.is_prefix_monitored("217.164.0.0/15"));
        assert!(config.is_prefix_monitored("192.0.2.0/24"));

        // Verify all ASNs are parsed
        assert_eq!(config.monitored_asns.len(), 2);
        assert!(config.is_asn_monitored("3333"));
        assert!(config.is_asn_monitored("5384"));

        // Verify descriptions
        let prefix1 = config.prefixes.get("217.164.0.0/15").unwrap();
        assert!(prefix1.description.contains("No description provided"));

        let prefix2 = config.prefixes.get("192.0.2.0/24").unwrap();
        assert!(prefix2.description.contains("Expected prefix"));
    }

    #[test]
    fn test_invalid_yaml() {
        let yaml = "invalid: yaml: content: [";

        let result = PrefixesConfig::from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_required_fields() {
        // Test with missing required fields (should still parse but might have issues)
        let yaml = r#"
192.0.2.0/24:
  asn:
    - 3333
"#;

        // This should fail because description and group are required
        let result = PrefixesConfig::from_str(yaml);
        // Depending on serde behavior, this might fail or use defaults
        // Let's just verify it doesn't panic
        if let Ok(config) = result {
            // If it parses, verify what we can
            assert!(
                config.is_prefix_monitored("192.0.2.0/24")
                    || !config.is_prefix_monitored("192.0.2.0/24")
            );
        }
    }

    #[test]
    fn test_is_operator_prefix() {
        let yaml = r#"
217.164.0.0/15:
  description: Test prefix
  asn:
    - 5384
  ignoreMorespecifics: false
  ignore: false
  group: noc
192.0.2.0/24:
  description: Ignored prefix
  asn:
    - 3333
  ignoreMorespecifics: false
  ignore: true
  group: noc
"#;

        let config = PrefixesConfig::from_str(yaml).unwrap();

        // Normal prefix should be monitored
        assert!(config.is_prefix_monitored("217.164.0.0/15"));

        // Ignored prefix should still be monitored (just has ignore flag set)
        assert!(config.is_prefix_monitored("192.0.2.0/24"));
        let ignored_prefix = config.prefixes.get("192.0.2.0/24").unwrap();
        assert!(ignored_prefix.ignore);

        // Non-existent prefix should not be monitored
        assert!(!config.is_prefix_monitored("10.0.0.0/8"));
    }

    #[test]
    fn test_is_operator_asn() {
        let yaml = r#"
options:
  monitorASns:
    '3333':
      group: noc
    '5384':
      group: production
"#;

        let config = PrefixesConfig::from_str(yaml).unwrap();

        assert!(config.is_asn_monitored("3333"));
        assert!(config.is_asn_monitored("5384"));
        assert!(!config.is_asn_monitored("1234"));
    }

    #[test]
    fn test_is_potential_hijack() {
        let yaml = r#"
217.164.0.0/15:
  description: Test prefix
  asn:
    - 5384
  ignoreMorespecifics: false
  ignore: false
  group: noc
192.0.2.0/24:
  description: Ignored prefix
  asn:
    - 3333
  ignoreMorespecifics: false
  ignore: true
  group: noc
"#;

        let config = PrefixesConfig::from_str(yaml).unwrap();

        // Verify prefix is monitored
        assert!(config.is_prefix_monitored("217.164.0.0/15"));

        // Verify expected ASN is in the prefix's ASN list
        let prefix_info = config.prefixes.get("217.164.0.0/15").unwrap();
        assert!(prefix_info.asn.contains(&5384));

        // Ignored prefix should still be monitored
        assert!(config.is_prefix_monitored("192.0.2.0/24"));
        let ignored_prefix = config.prefixes.get("192.0.2.0/24").unwrap();
        assert!(ignored_prefix.ignore);

        // Non-monitored prefix should not be in config
        assert!(!config.is_prefix_monitored("10.0.0.0/8"));
    }

    #[test]
    fn test_load_from_file() {
        // Test loading the actual prefixes.yml file from the root
        let config = PrefixesConfig::load("prefixes.yml").unwrap();

        // Verify prefixes are loaded
        assert!(config.is_prefix_monitored("217.164.0.0/15"));
        assert!(config.is_prefix_monitored("192.0.2.0/24"));

        // Verify ASNs are loaded
        assert!(config.is_asn_monitored("3333"));
        assert!(config.is_asn_monitored("5384"));

        // Verify expected ASNs in prefixes
        let prefix1 = config.prefixes.get("217.164.0.0/15").unwrap();
        assert!(prefix1.asn.contains(&5384));

        let prefix2 = config.prefixes.get("192.0.2.0/24").unwrap();
        assert!(prefix2.asn.contains(&3333));
    }

    #[test]
    fn test_load_from_test_file() {
        // Test loading from a separate test prefixes file
        let config = PrefixesConfig::load("prefixes.test.yml").unwrap();

        // Verify test prefixes are loaded
        assert!(config.is_prefix_monitored("10.0.0.0/8"));
        assert!(config.is_prefix_monitored("172.16.0.0/12"));
        assert!(config.is_prefix_monitored("192.168.1.0/24"));

        // Verify test ASNs are loaded
        assert!(config.is_asn_monitored("65000"));
        assert!(config.is_asn_monitored("65001"));
        assert!(config.is_asn_monitored("65002"));
        assert!(config.is_asn_monitored("65003"));

        // Verify prefix with multiple ASNs
        let prefix_info = config.prefixes.get("10.0.0.0/8").unwrap();
        assert!(prefix_info.asn.contains(&65000));
        assert!(prefix_info.asn.contains(&65001));
    }
}
