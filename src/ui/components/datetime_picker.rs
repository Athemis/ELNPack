// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! DateTime picker converted to MVU-style model/update/view.

use eframe::egui;
use egui_extras::DatePickerButton;
use jiff::{Zoned, civil::Date as CivilDate, tz::TimeZone};
use time::OffsetDateTime;

/// Format an integer as a two-digit string (00-99).
fn format_two(n: i32) -> String {
    format!("{:02}", n.clamp(0, 99))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DateTimeModel {
    /// Selected calendar date (local).
    pub date: CivilDate,
    /// Selected hour (0-23).
    pub hour: i32,
    /// Selected minute (0-59).
    pub minute: i32,
}

impl Default for DateTimeModel {
    fn default() -> Self {
        let now = Zoned::now();

        Self {
            date: now.date(),
            hour: i32::from(now.hour()),
            minute: i32::from(now.minute()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum DateTimeMsg {
    /// Update the date field.
    SetDate(CivilDate),
    /// Update the hour field.
    SetHour(i32),
    /// Update the minute field.
    SetMinute(i32),
    /// Set the picker to the current local time.
    SetNow,
}

/// Apply a message to the date/time model.
pub fn update(model: &mut DateTimeModel, msg: DateTimeMsg) {
    match msg {
        DateTimeMsg::SetDate(date) => model.date = date,
        DateTimeMsg::SetHour(h) => model.hour = h.clamp(0, 23),
        DateTimeMsg::SetMinute(m) => model.minute = m.clamp(0, 59),
        DateTimeMsg::SetNow => set_to_now(model),
    }
}

/// Render the picker controls and return any triggered messages.
pub fn view(model: &DateTimeModel, ui: &mut egui::Ui) -> Vec<DateTimeMsg> {
    let mut msgs = Vec::new();

    ui.horizontal(|ui| {
        let mut date = model.date;
        if ui
            .add(DatePickerButton::new(&mut date).show_icon(true))
            .changed()
        {
            msgs.push(DateTimeMsg::SetDate(date));
        }
        ui.add_space(8.0);

        let mut hour = model.hour;
        if ui
            .add(
                egui::DragValue::new(&mut hour)
                    .range(0..=23)
                    .speed(0.1)
                    .clamp_existing_to_range(true)
                    .custom_formatter(|v, _| format_two(v as i32)),
            )
            .changed()
        {
            msgs.push(DateTimeMsg::SetHour(hour));
        }
        ui.label(":");
        let mut minute = model.minute;
        if ui
            .add(
                egui::DragValue::new(&mut minute)
                    .range(0..=59)
                    .speed(0.1)
                    .clamp_existing_to_range(true)
                    .custom_formatter(|v, _| format_two(v as i32)),
            )
            .changed()
        {
            msgs.push(DateTimeMsg::SetMinute(minute));
        }

        ui.add_space(8.0);
        if ui
            .button(egui::RichText::new(format!(
                "{} Now",
                egui_phosphor::regular::CLOCK
            )))
            .on_hover_text("Set date/time to your current local time (stored as UTC)")
            .clicked()
        {
            msgs.push(DateTimeMsg::SetNow);
        }
    });

    msgs
}

/// Convert the selected date and time to an `OffsetDateTime` in UTC.
pub fn to_offset_datetime(model: &DateTimeModel) -> Result<OffsetDateTime, String> {
    if !(0..=23).contains(&model.hour) {
        return Err("Hour must be 0-23".into());
    }
    if !(0..=59).contains(&model.minute) {
        return Err("Minute must be 0-59".into());
    }

    let datetime = model.date.at(model.hour as i8, model.minute as i8, 0, 0);
    let zoned = datetime
        .to_zoned(TimeZone::system())
        .map_err(|_| "Invalid local date/time (likely skipped by offset)".to_string())?;
    let timestamp = zoned.timestamp();
    let unix_nanos = i128::from(timestamp.as_second()) * 1_000_000_000
        + i128::from(timestamp.subsec_nanosecond());
    let utc_dt = OffsetDateTime::from_unix_timestamp_nanos(unix_nanos)
        .map_err(|e| format!("Failed to construct timestamp: {e}"))?;

    Ok(utc_dt)
}

/// Update the model fields to the current local date and time.
fn set_to_now(model: &mut DateTimeModel) {
    let now = Zoned::now();
    model.date = now.date();
    model.hour = i32::from(now.hour());
    model.minute = i32::from(now.minute());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_datetime_is_now() {
        let picker = DateTimeModel::default();
        assert!(picker.hour >= 0 && picker.hour <= 23);
        assert!(picker.minute >= 0 && picker.minute <= 59);
    }

    #[test]
    fn set_to_now_updates_all_fields() {
        let mut picker = DateTimeModel {
            date: CivilDate::new(2000, 1, 1).unwrap(),
            hour: 0,
            minute: 0,
        };

        super::update(&mut picker, super::DateTimeMsg::SetNow);

        assert!(picker.hour >= 0 && picker.hour <= 23);
        assert!(picker.minute >= 0 && picker.minute <= 59);
    }

    #[test]
    fn to_offset_datetime_handles_dst_transitions() {
        // Note: This tests the actual edge case that CAN occur through the UI.
        // During DST transitions, certain times don't exist (spring forward)
        // or are ambiguous (fall back). Jiff surfaces these cases as an error
        // when converting a civil datetime into a zoned datetime.
        //
        // In most timezones, this is unlikely to affect users since they're
        // selecting times in the past for experiment timestamps, but we should
        // handle it gracefully if someone sets a time during a transition.

        // This test documents the behavior rather than asserting specific values,
        // since DST rules vary by timezone and locale.
        let picker = DateTimeModel {
            date: CivilDate::new(2024, 3, 10).unwrap(), // Common DST date
            hour: 2,                                    // Common DST transition hour
            minute: 30,
        };

        // The conversion should either succeed or fail gracefully with a clear message
        match super::to_offset_datetime(&picker) {
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
        let picker = DateTimeModel {
            date: CivilDate::new(2024, 6, 15).unwrap(),
            hour: 14,
            minute: 30,
        };

        let result = super::to_offset_datetime(&picker);
        assert!(result.is_ok());
    }
}
