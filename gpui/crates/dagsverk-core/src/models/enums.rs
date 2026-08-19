use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::DomainError;

macro_rules! persisted_enum {
    ($name:ident { $($variant:ident = $value:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(i32)]
        pub enum $name {
            $($variant = $value),+
        }

        impl TryFrom<i64> for $name {
            type Error = DomainError;

            fn try_from(value: i64) -> Result<Self, Self::Error> {
                match value {
                    $($value => Ok(Self::$variant),)+
                    value => Err(DomainError::UnknownEnumValue {
                        enum_name: stringify!($name),
                        value,
                    }),
                }
            }
        }

        impl From<$name> for i32 {
            fn from(value: $name) -> Self {
                value as i32
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_i32(*self as i32)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = i64::deserialize(deserializer)?;
                Self::try_from(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

persisted_enum!(WorkEntryStatus {
    Incomplete = 0,
    Worked = 1,
    Off = 2,
});
persisted_enum!(ThemePreference {
    System = 0,
    Light = 1,
    Dark = 2,
});
persisted_enum!(MonthViewPreference {
    Ledger = 0,
    Calendar = 1,
});
persisted_enum!(LanguagePreference {
    System = 0,
    English = 1,
    Swedish = 2,
});
persisted_enum!(WorkspaceType {
    Employment = 0,
    Contract = 1,
    Personal = 2,
});
persisted_enum!(ExportLanguagePreference {
    Swedish = 0,
    English = 1,
    System = 2,
});
persisted_enum!(OvertimeCompensationMode {
    CompTime = 0,
    Paid = 1,
});
persisted_enum!(OvertimeThresholdMode {
    FixedDailyHours = 0,
    ScheduledHours = 1,
});
persisted_enum!(CompensationRuleType {
    Overtime = 0,
    Ob = 1,
});
persisted_enum!(CompensationRateType {
    HourlyPremiumPercent = 0,
    FixedHourlyAmount = 1,
    FullTimeMonthlySalaryDivisor = 2,
});
persisted_enum!(OvertimeDayCategory {
    AllDays = 0,
    ScheduledWorkdays = 1,
    NonWorkdays = 2,
    Monday = 3,
    Tuesday = 4,
    Wednesday = 5,
    Thursday = 6,
    Friday = 7,
    Saturday = 8,
    Sunday = 9,
    PublicHolidays = 10,
    ScheduledWeekdays = 11,
    Weekends = 12,
    MajorHolidays = 13,
});
persisted_enum!(SalaryType {
    Hourly = 0,
    Monthly = 1,
});
persisted_enum!(HourlyPayBasis {
    DailyRegularHours = 0,
    MonthlyExpectedHours = 1,
});
persisted_enum!(ObOvertimeCombinationMode {
    ExcludeOb = 0,
    IncludeOb = 1,
});
persisted_enum!(TaxMode {
    Disabled = 0,
    PrimaryIncomeTaxTable = 1,
    SecondaryIncomeThirtyPercent = 2,
    ManualMonthlyDeduction = 3,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CurrencyPreference {
    #[serde(rename = "SEK")]
    Sek,
    #[serde(rename = "EUR")]
    Eur,
    #[serde(rename = "USD")]
    Usd,
    #[serde(rename = "GBP")]
    Gbp,
    #[serde(rename = "NOK")]
    Nok,
    #[serde(rename = "DKK")]
    Dkk,
}

impl TryFrom<i64> for CurrencyPreference {
    type Error = DomainError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Sek),
            1 => Ok(Self::Eur),
            2 => Ok(Self::Usd),
            3 => Ok(Self::Gbp),
            4 => Ok(Self::Nok),
            5 => Ok(Self::Dkk),
            value => Err(DomainError::UnknownEnumValue {
                enum_name: "CurrencyPreference",
                value,
            }),
        }
    }
}

impl From<CurrencyPreference> for i32 {
    fn from(value: CurrencyPreference) -> Self {
        match value {
            CurrencyPreference::Sek => 0,
            CurrencyPreference::Eur => 1,
            CurrencyPreference::Usd => 2,
            CurrencyPreference::Gbp => 3,
            CurrencyPreference::Nok => 4,
            CurrencyPreference::Dkk => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaxUnavailableReason {
    None,
    ManualDeductionNotConfigured,
    TaxYearNotBundled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateStatus {
    Unavailable,
    Idle,
    Checking,
    Available,
    Downloading,
    Ready,
    Current,
    Error,
}
