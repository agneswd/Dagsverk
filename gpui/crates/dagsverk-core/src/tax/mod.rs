use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::Deserialize;

use crate::models::{Money, TaxEstimate, TaxMode, TaxSettings, TaxUnavailableReason};

#[derive(Debug, Default)]
pub struct TaxEngine {
    years: BTreeMap<i32, BTreeMap<i32, Vec<TaxTableRange>>>,
}

impl TaxEngine {
    pub fn register_json(&mut self, json: &str) -> serde_json::Result<()> {
        let file: TaxTableFile = serde_json::from_str(json)?;
        let mut tables: BTreeMap<i32, Vec<TaxTableRange>> = BTreeMap::new();
        for range in file.ranges {
            tables.entry(range.table_number).or_default().push(range);
        }
        for ranges in tables.values_mut() {
            ranges.sort_by_key(|range| range.lower_bound);
        }
        self.years.insert(file.tax_year, tables);
        Ok(())
    }

    pub fn calculate(&self, gross_pay: Money, settings: &TaxSettings) -> TaxEstimate {
        let gross = gross_pay.decimal().max(Decimal::ZERO);
        match settings.mode {
            TaxMode::Disabled => available(gross, Decimal::ZERO),
            TaxMode::SecondaryIncomeThirtyPercent => {
                available(gross, (gross * Decimal::new(30, 2)).trunc().min(gross))
            }
            TaxMode::ManualMonthlyDeduction => match settings.manual_monthly_deduction {
                Some(deduction) => {
                    available(gross, deduction.decimal().max(Decimal::ZERO).min(gross))
                }
                None => unavailable(gross, TaxUnavailableReason::ManualDeductionNotConfigured),
            },
            TaxMode::PrimaryIncomeTaxTable => self.calculate_from_table(gross, settings),
        }
    }

    fn calculate_from_table(&self, gross: Decimal, settings: &TaxSettings) -> TaxEstimate {
        let Some(tables) = self.years.get(&settings.tax_year) else {
            return unavailable(gross, TaxUnavailableReason::TaxYearNotBundled);
        };
        if gross <= Decimal::ZERO {
            return available(gross, Decimal::ZERO);
        }
        let Some(ranges) = tables.get(&settings.table_number) else {
            return unavailable(gross, TaxUnavailableReason::TaxYearNotBundled);
        };
        let whole_krona = gross.floor();
        let Some(range) = find_range(ranges, whole_krona) else {
            return unavailable(gross, TaxUnavailableReason::TaxYearNotBundled);
        };
        let column = settings.column.clamp(1, 6) as usize - 1;
        let value = Decimal::from(*range.columns.get(column).unwrap_or(&0));
        let tax = if range.amount_kind == "%" {
            (whole_krona * value / Decimal::ONE_HUNDRED).floor()
        } else {
            value
        }
        .max(Decimal::ZERO)
        .min(gross);
        available(gross, tax)
    }
}

fn find_range(ranges: &[TaxTableRange], income: Decimal) -> Option<&TaxTableRange> {
    let mut low = 0;
    let mut high = ranges.len();
    while low < high {
        let middle = low + (high - low) / 2;
        let range = &ranges[middle];
        if income < Decimal::from(range.lower_bound) {
            high = middle;
        } else if income > Decimal::from(range.upper_bound) {
            low = middle + 1;
        } else {
            return Some(range);
        }
    }
    None
}

fn available(gross: Decimal, tax: Decimal) -> TaxEstimate {
    TaxEstimate {
        gross_pay: Money::new(gross),
        preliminary_tax: Some(Money::new(tax)),
        estimated_net_pay: Some(Money::new(gross - tax)),
        unavailable_reason: TaxUnavailableReason::None,
        is_available: true,
    }
}

fn unavailable(gross: Decimal, reason: TaxUnavailableReason) -> TaxEstimate {
    TaxEstimate {
        gross_pay: Money::new(gross),
        preliminary_tax: None,
        estimated_net_pay: None,
        unavailable_reason: reason,
        is_available: false,
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TaxTableFile {
    tax_year: i32,
    ranges: Vec<TaxTableRange>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TaxTableRange {
    table_number: i32,
    lower_bound: i64,
    upper_bound: i64,
    amount_kind: String,
    columns: Vec<i64>,
}
