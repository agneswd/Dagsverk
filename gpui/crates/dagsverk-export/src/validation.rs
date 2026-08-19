use std::path::Path;

use dagsverk_core::models::{ReportExportRequest, WorkEntryStatus, YearMonth};

use crate::{ExportError, Result};

pub fn validate_request(
    request: &ReportExportRequest,
    output: &Path,
    extension: &'static str,
) -> Result<()> {
    let month =
        YearMonth::new(request.year, request.month).map_err(|_| ExportError::InvalidMonth)?;
    if output
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case(extension))
    {
        return Err(ExportError::InvalidExtension {
            expected: extension,
            path: output.to_owned(),
        });
    }
    let mut dates = std::collections::BTreeSet::new();
    for entry in &request.entries {
        if !month.contains(entry.date) {
            return Err(ExportError::EntryOutsideMonth(entry.date.to_string()));
        }
        if !dates.insert(entry.date) {
            return Err(ExportError::DuplicateDate(entry.date.to_string()));
        }
        if entry.status == WorkEntryStatus::Worked {
            if entry.start_time.is_none() || entry.end_time.is_none() {
                return Err(ExportError::InvalidTime(entry.date.to_string()));
            }
            if entry.lunch_minutes.value() < 0 {
                return Err(ExportError::InvalidLunch(entry.date.to_string()));
            }
        }
    }
    Ok(())
}
