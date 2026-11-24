//! Five-field UTC cron schedules for workflow triggers.

use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("invalid cron expression: {0}")]
pub struct CronError(pub String);

#[derive(Debug, Clone)]
pub struct CronSchedule {
    minute: Field,
    hour: Field,
    day: Field,
    month: Field,
    weekday: Field,
}

#[derive(Debug, Clone)]
struct Field {
    values: Vec<bool>,
    wildcard: bool,
}

impl CronSchedule {
    /// Parse standard five-field cron syntax. All dates are interpreted as UTC.
    pub fn parse(expression: &str) -> Result<Self, CronError> {
        let fields: Vec<_> = expression.split_whitespace().collect();
        if fields.len() != 5 {
            return Err(CronError(
                "expected five fields: minute hour day month weekday".into(),
            ));
        }
        Ok(Self {
            minute: Field::parse(fields[0], 0, 59, false)?,
            hour: Field::parse(fields[1], 0, 23, false)?,
            day: Field::parse(fields[2], 1, 31, false)?,
            month: Field::parse(fields[3], 1, 12, false)?,
            weekday: Field::parse(fields[4], 0, 7, true)?,
        })
    }

    pub fn matches_unix_seconds(&self, timestamp: u64) -> bool {
        let minutes = timestamp / 60;
        let minute = (minutes % 60) as usize;
        let hour = ((minutes / 60) % 24) as usize;
        let days_since_epoch = (minutes / (60 * 24)) as i64;
        let (_, month, day) = civil_from_days(days_since_epoch);
        let weekday = (days_since_epoch + 4).rem_euclid(7) as usize;

        if !self.minute.contains(minute)
            || !self.hour.contains(hour)
            || !self.month.contains(month as usize)
        {
            return false;
        }
        let day_match = self.day.contains(day as usize);
        let weekday_match = self.weekday.contains(weekday);
        let calendar_match = match (self.day.wildcard, self.weekday.wildcard) {
            (false, false) => day_match || weekday_match,
            _ => day_match && weekday_match,
        };
        calendar_match
    }
}

impl Field {
    fn parse(text: &str, min: usize, max: usize, sunday_alias: bool) -> Result<Self, CronError> {
        let wildcard = text.starts_with('*');
        let mut values = vec![false; max + 1];
        for item in text.split(',') {
            let (base, step) = match item.split_once('/') {
                Some((base, step)) => {
                    let step = step
                        .parse::<usize>()
                        .ok()
                        .filter(|step| *step > 0)
                        .ok_or_else(|| CronError(format!("invalid step '{step}'")))?;
                    (base, step)
                }
                None => (item, 1),
            };
            let (start, end) = if base == "*" {
                (min, max)
            } else if let Some((start, end)) = base.split_once('-') {
                (parse_number(start, min, max)?, parse_number(end, min, max)?)
            } else {
                let value = parse_number(base, min, max)?;
                (value, value)
            };
            if start > end {
                return Err(CronError(format!("descending range '{base}'")));
            }
            for value in (start..=end).step_by(step) {
                values[value] = true;
                if sunday_alias && value == 7 {
                    values[0] = true;
                }
            }
        }
        Ok(Self { values, wildcard })
    }

    fn contains(&self, value: usize) -> bool {
        self.values.get(value).copied().unwrap_or(false)
    }
}

fn parse_number(text: &str, min: usize, max: usize) -> Result<usize, CronError> {
    let value = text
        .parse::<usize>()
        .map_err(|_| CronError(format!("invalid field value '{text}'")))?;
    if value < min || value > max {
        return Err(CronError(format!("field value '{text}' is out of range")));
    }
    Ok(value)
}

// Gregorian civil date conversion from days relative to 1970-01-01.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month as u32, day as u32)
}

/// Tracks the last minute dispatched for each scheduled workflow to avoid
/// duplicate runs when the clock is checked more than once in a minute.
#[derive(Debug, Default)]
pub struct ScheduleDispatcher {
    schedules: HashMap<String, CronSchedule>,
    last_fired_minute: HashMap<String, u64>,
}

impl ScheduleDispatcher {
    pub fn register(
        &mut self,
        workflow_id: impl Into<String>,
        cron: &str,
    ) -> Result<(), CronError> {
        let schedule = CronSchedule::parse(cron)?;
        self.schedules.insert(workflow_id.into(), schedule);
        Ok(())
    }

    pub fn due(&mut self, unix_seconds: u64) -> Vec<String> {
        let minute = unix_seconds / 60;
        let mut due = Vec::new();
        for (workflow_id, schedule) in &self.schedules {
            if schedule.matches_unix_seconds(unix_seconds)
                && self.last_fired_minute.get(workflow_id) != Some(&minute)
            {
                self.last_fired_minute.insert(workflow_id.clone(), minute);
                due.push(workflow_id.clone());
            }
        }
        due.sort();
        due
    }
}
