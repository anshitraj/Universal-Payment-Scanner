use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{Category, Maturity, SchemeMetadata, Support};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CapabilityPolicy {
    pub preset: Preset,
    pub schemes: BTreeMap<String, bool>,
    pub categories: BTreeMap<String, bool>,
    pub countries: BTreeMap<String, BTreeMap<String, bool>>,
    pub max_payload_bytes: usize,
    pub accept: AcceptPolicy,
}

/// Fine-grained accept policy below the scheme/category level of the rest of `CapabilityPolicy` -
/// today just crypto network/asset allow-listing, since that's the one dimension a scheme/category
/// toggle can't express (a host that accepts `crypto` still needs to say *which* chains and tokens
/// it can actually settle).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct AcceptPolicy {
    /// Network id (as reported in `PaymentIntent.network`, e.g. `"base"`, `"solana"`) -> allowed
    /// asset symbols on that network. An empty symbol list means the network is allowed for any
    /// asset. An empty map (the default) means no crypto-specific restriction is applied at all -
    /// crypto intents are governed only by the usual scheme/category policy, same as before this
    /// field existed.
    pub crypto: BTreeMap<String, Vec<String>>,
}

/// Why `CapabilityPolicy::crypto_check` rejected a crypto `PaymentIntent`, carrying enough detail
/// to build `Support.details` for the host - see `to_support`.
pub(crate) enum CryptoRejection {
    NetworkUnsupported {
        network: String,
        allowed_networks: Vec<String>,
    },
    /// The network is allowed, but the asset's identity can't be confirmed from the QR alone (e.g.
    /// an ERC-20 transfer where only the contract address is known - see `Asset.symbol`'s doc
    /// comment on not guessing token identity from a contract address). Fails closed rather than
    /// silently letting an unverified asset through an explicit accept-list.
    AssetUnverifiable {
        network: String,
        allowed_assets: Vec<String>,
        contract: Option<String>,
    },
    AssetUnsupported {
        network: String,
        asset: String,
        allowed_assets: Vec<String>,
    },
}

impl CryptoRejection {
    pub(crate) fn to_support(&self) -> Support {
        let (reason, message, message_key, details) = match self {
            CryptoRejection::NetworkUnsupported {
                network,
                allowed_networks,
            } => (
                crate::ErrorCode::NetworkUnsupported,
                format!("{network} is not an accepted network for this application."),
                "scanner.network_unsupported",
                json!({ "network": network, "allowedNetworks": allowed_networks }),
            ),
            CryptoRejection::AssetUnverifiable {
                network,
                allowed_assets,
                contract,
            } => (
                crate::ErrorCode::AssetUnverifiable,
                "This asset's identity could not be verified from the QR payload alone; \
                 configure a token registry or resolver to accept it."
                    .to_string(),
                "scanner.asset_unverifiable",
                json!({ "network": network, "allowedAssets": allowed_assets, "contract": contract }),
            ),
            CryptoRejection::AssetUnsupported {
                network,
                asset,
                allowed_assets,
            } => (
                crate::ErrorCode::AssetUnsupported,
                format!("{asset} on {network} is not an accepted asset for this application."),
                "scanner.asset_unsupported",
                json!({ "network": network, "asset": asset, "allowedAssets": allowed_assets }),
            ),
        };
        Support {
            enabled: false,
            reason: Some(reason),
            message: Some(message),
            message_key: Some(message_key.into()),
            details: Some(details),
        }
    }
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
            accept: AcceptPolicy::default(),
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

    /// Checks a crypto intent's `network`/asset `symbol` against `accept.crypto`. An empty
    /// `accept.crypto` map (the default) always passes - see `AcceptPolicy::crypto`'s doc comment.
    pub(crate) fn crypto_check(
        &self,
        network: &str,
        symbol: Option<&str>,
        contract: Option<&str>,
    ) -> Result<(), CryptoRejection> {
        if self.accept.crypto.is_empty() {
            return Ok(());
        }
        let Some(allowed_assets) = self.accept.crypto.get(network) else {
            return Err(CryptoRejection::NetworkUnsupported {
                network: network.to_string(),
                allowed_networks: self.accept.crypto.keys().cloned().collect(),
            });
        };
        if allowed_assets.is_empty() {
            return Ok(());
        }
        match symbol {
            None => Err(CryptoRejection::AssetUnverifiable {
                network: network.to_string(),
                allowed_assets: allowed_assets.clone(),
                contract: contract.map(str::to_string),
            }),
            Some(symbol)
                if allowed_assets
                    .iter()
                    .any(|a| a.eq_ignore_ascii_case(symbol)) =>
            {
                Ok(())
            }
            Some(symbol) => Err(CryptoRejection::AssetUnsupported {
                network: network.to_string(),
                asset: symbol.to_string(),
                allowed_assets: allowed_assets.clone(),
            }),
        }
    }
}
