use std::{borrow::Cow, collections::BTreeMap, sync::OnceLock};

use crate::models::LanguagePreference;

const ENGLISH_JSON: &str = include_str!("../../../../assets/i18n/en.json");
const SWEDISH_JSON: &str = include_str!("../../../../assets/i18n/sv.json");

static ENGLISH: OnceLock<BTreeMap<String, String>> = OnceLock::new();
static SWEDISH: OnceLock<BTreeMap<String, String>> = OnceLock::new();

pub fn resources(language: LanguagePreference) -> &'static BTreeMap<String, String> {
    match language {
        LanguagePreference::Swedish => SWEDISH.get_or_init(|| parse(SWEDISH_JSON)),
        LanguagePreference::System | LanguagePreference::English => {
            ENGLISH.get_or_init(|| parse(ENGLISH_JSON))
        }
    }
}

pub fn translate<'a>(language: LanguagePreference, key: &'a str) -> Cow<'a, str> {
    resources(language)
        .get(key)
        .map_or_else(|| Cow::Borrowed(key), |value| Cow::Owned(value.clone()))
}

fn parse(json: &str) -> BTreeMap<String, String> {
    serde_json::from_str(json).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{ENGLISH_JSON, SWEDISH_JSON, parse};

    #[test]
    fn generated_catalogs_have_matching_nonempty_keys_and_placeholders() {
        let english = parse(ENGLISH_JSON);
        let swedish = parse(SWEDISH_JSON);
        assert_eq!(
            english.keys().collect::<Vec<_>>(),
            swedish.keys().collect::<Vec<_>>()
        );
        assert!(!english.is_empty());
        for (key, english_value) in &english {
            let swedish_value = &swedish[key];
            assert!(!english_value.is_empty(), "empty English value for {key}");
            assert!(!swedish_value.is_empty(), "empty Swedish value for {key}");
            assert_eq!(
                placeholders(english_value),
                placeholders(swedish_value),
                "{key}"
            );
        }
    }

    fn placeholders(value: &str) -> BTreeSet<&str> {
        let mut result = BTreeSet::new();
        let mut rest = value;
        while let Some(start) = rest.find('{') {
            rest = &rest[start + 1..];
            let Some(end) = rest.find('}') else {
                break;
            };
            result.insert(&rest[..end]);
            rest = &rest[end + 1..];
        }
        result
    }
}
