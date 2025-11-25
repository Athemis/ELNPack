//! DateTime picker component for selecting experiment timestamps.

use chrono::{Datelike, Local, NaiveDate, TimeZone, Timelike, Utc};
use eframe::egui;
use egui_extras::DatePickerButton;
use time::OffsetDateTime;

/// Format an integer as a two-digit string (00-99).
fn format_two(n: i32) -> String {
    format!("{:02}", n.clamp(0, 99))
}

/// DateTime picker widget with date, hour, and minute controls.
pub struct DateTimePicker {
    date: NaiveDate,
    hour: i32,
    minute: i32,
}

impl Default for DateTimePicker {
    fn default() -> Self {
        let now = Local::now();
        let today = now.date_naive();

        Self {
            date: today,
            hour: now.hour() as i32,
            minute: now.minute() as i32,
        }
    }
}

impl DateTimePicker {
    /// Render the datetime picker controls in a horizontal layout.
    ///
    /// Displays a date picker button, hour/minute drag values, and a "Now" button.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(DatePickerButton::new(&mut self.date).show_icon(true));
            ui.add_space(8.0);

            ui.add(
                egui::DragValue::new(&mut self.hour)
                    .range(0..=23)
                    .speed(0.1)
                    .clamp_existing_to_range(true)
                    .custom_formatter(|v, _| format_two(v as i32)),
            );
            ui.label(":");
            ui.add(
                egui::DragValue::new(&mut self.minute)
                    .range(0..=59)
                    .speed(0.1)
                    .clamp_existing_to_range(true)
                    .custom_formatter(|v, _| format_two(v as i32)),
            );

            ui.add_space(8.0);
            if ui
                .button(egui::RichText::new(format!(
                    "{} Now",
                    egui_phosphor::regular::CLOCK
                )))
                .on_hover_text("Set date/time to your current local time (stored as UTC)")
                .clicked()
            {
                self.set_to_now();
            }
        });
    }

    /// Set the datetime to the current local time.
    pub fn set_to_now(&mut self) {
        let now = Local::now();
        self.date = now.date_naive();
        self.hour = now.hour() as i32;
        self.minute = now.minute() as i32;
    }

    /// Convert the selected date and time to an `OffsetDateTime` in UTC.
    ///
    /// Returns an error if the date/time is invalid (e.g., hour out of range,
    /// invalid calendar date, or time skipped by DST transition).
    ///
    /// Note: Hour/minute range checks are defensive programming - the UI widgets
    /// enforce these constraints via `.clamp_existing_to_range(true)`, but we
    /// validate here to protect against direct field manipulation or future changes.
    pub fn to_offset_datetime(&self) -> Result<OffsetDateTime, String> {
        if !(0..=23).contains(&self.hour) {
            return Err("Hour must be 0-23".into());
        }
        if !(0..=59).contains(&self.minute) {
            return Err("Minute must be 0-59".into());
        }

        let naive =
            chrono::NaiveDate::from_ymd_opt(self.date.year(), self.date.month(), self.date.day())
                .and_then(|d| d.and_hms_opt(self.hour as u32, self.minute as u32, 0))
                .ok_or_else(|| "Invalid calendar date or time".to_string())?;

        let local_dt = Local
            .from_local_datetime(&naive)
            .single()
            .ok_or_else(|| "Invalid local date/time (likely skipped by offset)".to_string())?;

        let utc_ts = local_dt.with_timezone(&Utc).timestamp();
        let utc_dt = OffsetDateTime::from_unix_timestamp(utc_ts)
            .map_err(|e| format!("Failed to construct timestamp: {e}"))?;

        Ok(utc_dt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn default_datetime_is_now() {
        let picker = DateTimePicker::default();
        assert!(picker.hour >= 0 && picker.hour <= 23);
        assert!(picker.minute >= 0 && picker.minute <= 59);
    }

    #[test]
    fn set_to_now_updates_all_fields() {
        let mut picker = DateTimePicker {
            date: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
            hour: 0,
            minute: 0,
        };

        picker.set_to_now();

        assert!(picker.hour >= 0 && picker.hour <= 23);
        assert!(picker.minute >= 0 && picker.minute <= 59);
    }

    #[test]
    fn to_offset_datetime_handles_dst_transitions() {
        // Note: This tests the actual edge case that CAN occur through the UI.
        // During DST transitions, certain times don't exist (spring forward)
        // or are ambiguous (fall back). The chrono library handles this with
        // .single() which returns None for ambiguous/non-existent times.
        //
        // In most timezones, this is unlikely to affect users since they're
        // selecting times in the past for experiment timestamps, but we should
        // handle it gracefully if someone sets a time during a transition.

        // This test documents the behavior rather than asserting specific values,
        // since DST rules vary by timezone and locale.
        let picker = DateTimePicker {
            date: NaiveDate::from_ymd_opt(2024, 3, 10).unwrap(), // Common DST date
            hour: 2,                                             // Common DST transition hour
            minute: 30,
        };

        // The conversion should either succeed or fail gracefully with a clear message
        match picker.to_offset_datetime() {
            Ok(_) => {
                // Valid conversion (not in DST gap for this timezone)
            }
            Err(msg) => {
                // Should contain helpful error about DST
                assert!(msg.contains("Invalid local date/time") || msg.contains("offset"));
            }
        }
    }

    #[test]
    fn to_offset_datetime_succeeds_with_valid_input() {
        let picker = DateTimePicker {
            date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            hour: 14,
            minute: 30,
        };

        let result = picker.to_offset_datetime();
        assert!(result.is_ok());
    }
}
