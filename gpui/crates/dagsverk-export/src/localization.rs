use dagsverk_core::models::{ExportLanguagePreference, LanguagePreference, ReportExportRequest};

pub fn is_english(request: &ReportExportRequest, system: LanguagePreference) -> bool {
    request.language == ExportLanguagePreference::English
        || (request.language == ExportLanguagePreference::System
            && system != LanguagePreference::Swedish)
}

pub fn text<'a>(english: bool, english_text: &'a str, swedish_text: &'a str) -> &'a str {
    if english { english_text } else { swedish_text }
}

pub fn month_title(request: &ReportExportRequest, english: bool) -> String {
    const ENGLISH: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    const SWEDISH: [&str; 12] = [
        "januari",
        "februari",
        "mars",
        "april",
        "maj",
        "juni",
        "juli",
        "augusti",
        "september",
        "oktober",
        "november",
        "december",
    ];
    let names = if english { ENGLISH } else { SWEDISH };
    format!("{} {}", names[request.month as usize - 1], request.year)
}
