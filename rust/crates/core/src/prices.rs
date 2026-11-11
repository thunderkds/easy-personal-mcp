//! Dated API price table -> API-equivalent cost. A model not in the table abstains (no cost).
//!
//! Port of `src/easy_personal/core/prices.py`. The file is not YAML: an `as_of:` line, a `models:` line and
//! one `name: key=value ...` line per model, read by hand (no YAML crate). Like Python, a malformed model
//! line is an error, never skipped; unknown top-level keys and indented lines outside `models:` are ignored.

use std::collections::HashMap;
use std::fmt;

/// The table Python ships (read by path at build time; it moves with the data at the Python sunset, P6).
const SHIPPED: &str = include_str!("../../../../src/easy_personal/data/prices.yaml");
const PER: f64 = 1_000_000.0;

/// USD per million tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Price {
    pub input: f64,
    pub output: f64,
    pub cache_write: f64,
    pub cache_read: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PriceTable {
    pub as_of: String,
    models: HashMap<String, Price>,
}

/// A bad model line. Names the 1-based line and the problem, never the line's text.
#[derive(Debug, PartialEq)]
pub struct PriceError {
    pub line: usize,
    pub reason: &'static str,
}

impl fmt::Display for PriceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "price table line {}: {}", self.line, self.reason)
    }
}

impl std::error::Error for PriceError {}

impl PriceTable {
    /// The table shipped with the Python app.
    pub fn shipped() -> Result<PriceTable, PriceError> {
        PriceTable::parse(SHIPPED)
    }

    pub fn parse(text: &str) -> Result<PriceTable, PriceError> {
        let (mut as_of, mut in_models) = (String::new(), false);
        let mut models = HashMap::new();
        for (n, line) in text.lines().enumerate() {
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            if !line.starts_with(' ') {
                let (key, value) = line.split_once(':').unwrap_or((line, ""));
                in_models = key == "models";
                if key == "as_of" {
                    as_of = value.trim().to_string();
                }
            } else if in_models {
                let (name, rest) = line.trim().split_once(':').unwrap_or((line.trim(), ""));
                let price = parse_price(rest).map_err(|reason| PriceError {
                    line: n + 1,
                    reason,
                })?;
                models.insert(name.to_string(), price);
            }
        }
        Ok(PriceTable { as_of, models })
    }

    /// The price of a model, or `None` when it is not in the table (abstain, never guess).
    pub fn price(&self, model: &str) -> Option<&Price> {
        self.models.get(model)
    }

    /// API-equivalent USD of one message; `None` (abstain) for an empty or unlisted model.
    pub fn cost(
        &self,
        model: Option<&str>,
        input: i64,
        output: i64,
        cache_write: i64,
        cache_read: i64,
    ) -> Option<f64> {
        let price = self.price(model.filter(|m| !m.is_empty())?)?;
        Some(
            (input as f64 * price.input
                + output as f64 * price.output
                + cache_write as f64 * price.cache_write
                + cache_read as f64 * price.cache_read)
                / PER,
        )
    }
}

/// `input=5 output=25 cache_write=6.25 cache_read=0.5` -> exactly those four keys (a later duplicate wins).
fn parse_price(rest: &str) -> Result<Price, &'static str> {
    let mut fields: [Option<f64>; 4] = [None; 4];
    for item in rest.split_whitespace() {
        let mut parts = item.split('=');
        let (Some(key), Some(value), None) = (parts.next(), parts.next(), parts.next()) else {
            return Err("an item is not key=value");
        };
        let slot = match key {
            "input" => 0,
            "output" => 1,
            "cache_write" => 2,
            "cache_read" => 3,
            _ => return Err("unknown price key"),
        };
        fields[slot] = Some(
            value
                .parse::<f64>()
                .map_err(|_| "a price is not a number")?,
        );
    }
    match fields {
        [
            Some(input),
            Some(output),
            Some(cache_write),
            Some(cache_read),
        ] => Ok(Price {
            input,
            output,
            cache_write,
            cache_read,
        }),
        _ => Err("a price key is missing"),
    }
}
