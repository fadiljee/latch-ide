//! Tabel harga model LLM (`pricing.toml`) dan kalkulator estimasi biaya (T-203).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::ConfigError;

/// Harga per 1 juta token (USD) untuk satu model LLM.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Biaya per 1.000.000 token input (prompt) dalam USD.
    pub input_per_1m_usd: f64,
    /// Biaya per 1.000.000 token output (completion) dalam USD.
    pub output_per_1m_usd: f64,
}

/// Hasil kalkulasi estimasi biaya satu panggilan LLM.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CostEstimate {
    /// Estimasi biaya dalam USD (`0.0` bila model tidak dikenal).
    pub cost_usd: f64,
    /// `true` bila model cocok pada tabel harga.
    pub known_model: bool,
}

/// Kumpulan tarif model LLM yang dimuat dari `pricing.toml` + bawaan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PricingTable {
    /// Peta nama model (atau prefiks model) -> tarif USD per 1M token.
    pub models: BTreeMap<String, ModelPricing>,
}

impl Default for PricingTable {
    fn default() -> Self {
        let mut models = BTreeMap::new();
        // Tarif acuan umum (dapat ditimpa lewat ~/.config/latch/pricing.toml)
        let builtins: [(&str, f64, f64); 12] = [
            ("gpt-4o", 2.50, 10.00),
            ("gpt-4o-mini", 0.15, 0.60),
            ("gpt-4.1", 2.00, 8.00),
            ("gpt-4.1-mini", 0.40, 1.60),
            ("o3-mini", 1.10, 4.40),
            ("claude-3-5-sonnet", 3.00, 15.00),
            ("claude-3-7-sonnet", 3.00, 15.00),
            ("claude-sonnet-4", 3.00, 15.00),
            ("claude-opus-4", 15.00, 75.00),
            ("claude-3-5-haiku", 0.80, 4.00),
            ("gemini-2.5-pro", 1.25, 10.00),
            ("gemini-2.5-flash", 0.15, 0.60),
        ];
        for (name, input_per_1m_usd, output_per_1m_usd) in builtins {
            models.insert(
                name.to_string(),
                ModelPricing {
                    input_per_1m_usd,
                    output_per_1m_usd,
                },
            );
        }
        Self { models }
    }
}

impl PricingTable {
    /// Mencari tarif model berdasarkan kecocokan persis atau prefiks terpanjang
    /// (mis. `claude-3-5-sonnet-20241022` -> `claude-3-5-sonnet`).
    pub fn lookup(&self, model: &str) -> Option<ModelPricing> {
        let normalized = model.trim().to_ascii_lowercase();
        if let Some(exact) = self.models.get(&normalized) {
            return Some(*exact);
        }
        // Cari kecocokan prefiks terpanjang yang dibatasi '-' atau ':'
        let mut best: Option<(usize, ModelPricing)> = None;
        for (key, pricing) in &self.models {
            let key_lc = key.to_ascii_lowercase();
            if normalized == key_lc {
                return Some(*pricing);
            }
            if let Some(rest) = normalized.strip_prefix(&key_lc) {
                if rest.starts_with('-') || rest.starts_with(':') || rest.starts_with('.') {
                    let len = key_lc.len();
                    if best.is_none_or(|(best_len, _)| len > best_len) {
                        best = Some((len, *pricing));
                    }
                }
            }
        }
        best.map(|(_, pricing)| pricing)
    }

    /// Menghitung estimasi biaya USD berdasarkan jumlah token prompt dan completion.
    pub fn estimate_cost(
        &self,
        model: Option<&str>,
        prompt_tokens: i64,
        completion_tokens: i64,
    ) -> CostEstimate {
        let Some(model_name) = model else {
            return CostEstimate {
                cost_usd: 0.0,
                known_model: false,
            };
        };
        let Some(pricing) = self.lookup(model_name) else {
            return CostEstimate {
                cost_usd: 0.0,
                known_model: false,
            };
        };
        let in_tok = prompt_tokens.max(0) as f64;
        let out_tok = completion_tokens.max(0) as f64;
        let cost =
            (in_tok * pricing.input_per_1m_usd + out_tok * pricing.output_per_1m_usd) / 1_000_000.0;
        CostEstimate {
            cost_usd: cost,
            known_model: true,
        }
    }
}

/// Memuat `pricing.toml` dari disk dan menggabungkannya dengan tabel bawaan.
pub fn load_pricing_table(path: &Path) -> Result<PricingTable, ConfigError> {
    if !path.exists() {
        return Ok(PricingTable::default());
    }
    let content = fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.display().to_string(),
        source,
    })?;
    parse_pricing_table_str(&content, &path.display().to_string())
}

/// Mem-parse string `pricing.toml` lalu menggabungkannya dengan tarif bawaan.
pub fn parse_pricing_table_str(src: &str, path_label: &str) -> Result<PricingTable, ConfigError> {
    let custom: PricingTable = toml::from_str(src).map_err(|err| ConfigError::Toml {
        path: path_label.to_string(),
        line: 1,
        column: 1,
        field: "models".to_string(),
        message: err.message().to_string(),
    })?;
    let mut merged = PricingTable::default();
    for (name, pricing) in custom.models {
        if pricing.input_per_1m_usd < 0.0
            || pricing.output_per_1m_usd < 0.0
            || pricing.input_per_1m_usd.is_nan()
            || pricing.output_per_1m_usd.is_nan()
        {
            return Err(ConfigError::Validation {
                path: path_label.to_string(),
                line: 1,
                field: format!("models.{name}"),
                message: "pricing values must be non-negative numbers".to_string(),
            });
        }
        merged.models.insert(name, pricing);
    }
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::{parse_pricing_table_str, PricingTable};

    #[test]
    fn calculates_cost_accurately_for_exact_and_prefixed_models() {
        let table = PricingTable::default();
        let est = table.estimate_cost(Some("claude-3-5-sonnet-20241022"), 100_000, 20_000);
        assert!(est.known_model);
        // 0.1 * 3.0 + 0.02 * 15.0 = 0.30 + 0.30 = 0.60 USD
        assert!((est.cost_usd - 0.60).abs() < 1e-6);

        let unknown = table.estimate_cost(Some("mystery-model-x"), 10_000, 5_000);
        assert!(!unknown.known_model);
        assert_eq!(unknown.cost_usd, 0.0);
    }

    #[test]
    fn overrides_and_adds_models_from_toml() {
        let table = parse_pricing_table_str(
            r#"
[models.custom-local-llm]
input_per_1m_usd = 0.5
output_per_1m_usd = 1.5
"#,
            "pricing.toml",
        )
        .unwrap();
        let est = table.estimate_cost(Some("custom-local-llm"), 1_000_000, 500_000);
        assert!(est.known_model);
        assert!((est.cost_usd - 1.25).abs() < 1e-6);
    }
}
