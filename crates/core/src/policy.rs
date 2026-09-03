use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{Category, Maturity, SchemeMetadata};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CapabilityPolicy {
    pub preset: Preset,
    pub schemes: BTreeMap<String, bool>,
    pub categories: BTreeMap<String, bool>,
    pub countries: BTreeMap<String, BTreeMap<String, bool>>,
    pub max_payload_bytes: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Preset {
    India,
    AsiaPacific,
    BankingOnly,
    CryptoOnly,
    #[default]
    AllStable,
    All,
}

/// Countries covered by `Preset::AsiaPacific`. Mirrors `Preset::India`'s country-list matching
/// (schemes with no `countries` entry, like Bitcoin, are not included by either).
const ASIA_PACIFIC_COUNTRIES: &[&str] = &[
    "IN", "TH", "VN", "SG", "MY", "ID", "PH", "HK", "NP", "KH", "LK", "BD", "PK", "MM", "LA", "JP",
    "TW", "KR", "CN",
];

impl Default for CapabilityPolicy {
    fn default() -> Self {
        Self {
            preset: Preset::AllStable,
            schemes: BTreeMap::new(),
            categories: BTreeMap::new(),
            countries: BTreeMap::new(),
            max_payload_bytes: crate::DEFAULT_MAX_PAYLOAD_BYTES,
        }
    }
}

impl CapabilityPolicy {
    pub fn preset(preset: Preset) -> Self {
        Self {
            preset,
            ..Self::default()
        }
    }

    pub fn is_enabled(&self, metadata: &SchemeMetadata) -> bool {
        let preset_enabled = match self.preset {
            Preset::All => true,
            Preset::AllStable => matches!(metadata.maturity, Maturity::Stable | Maturity::Beta),
            Preset::India => metadata.countries.iter().any(|country| country == "IN"),
            Preset::AsiaPacific => metadata
                .countries
                .iter()
                .any(|country| ASIA_PACIFIC_COUNTRIES.contains(&country.as_str())),
            Preset::BankingOnly => {
                matches!(metadata.category, Category::BankTransfer | Category::Card)
            }
            Preset::CryptoOnly => metadata.category == Category::Crypto,
        };
        let category_key = match metadata.category {
            Category::BankTransfer => "bank_transfer",
            Category::Wallet => "wallet",
            Category::Card => "card",
            Category::Crypto => "crypto",
            Category::PaymentLink => "payment_link",
            Category::Unknown => "unknown",
        };
        let category_enabled = self
            .categories
            .get(category_key)
            .copied()
            .unwrap_or(preset_enabled);
        let country_override = metadata.countries.iter().find_map(|country| {
            self.countries
                .get(country)
                .and_then(|schemes| schemes.get(&metadata.id))
                .copied()
        });
        self.schemes
            .get(&metadata.id)
            .copied()
            .or(country_override)
            .unwrap_or(category_enabled)
    }
}
