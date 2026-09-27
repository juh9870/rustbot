use chrono::{DateTime, TimeDelta, Utc};
use regex_macro::checked_regex;

pub fn parse_ts(ts: &str) -> Result<(DateTime<Utc>, TimeDelta), String> {
    let delta = parse_ts_inner(ts)?;
    let date = Utc::now() + delta;
    Ok((date, delta))
}

pub fn format_date(date: DateTime<Utc>) -> String {
    date.format("%Y-%m-%d %H:%M:%S UTC+0").to_string()
}

pub fn format_duration(td: TimeDelta) -> String {
    fn pluralize(num: i64, singular: &str, plural: &str) -> String {
        match num {
            0 => "".to_owned(),
            1 => format!("1 {} ", singular),
            _ => format!("{} {} ", num, plural),
        }
    }

    let mut bulk = String::new();
    bulk.push_str(&pluralize(td.num_weeks(), "week", "weeks"));
    bulk.push_str(&pluralize(td.num_days() % 7, "day", "days"));
    bulk.push_str(&pluralize(td.num_hours() % 24, "hour", "hours"));
    bulk.push_str(&pluralize(td.num_minutes() % 60, "minute", "minutes"));
    if td.num_minutes() < 1 {
        bulk.push_str(&pluralize(td.num_seconds() % 60, "second", "seconds"));
    }

    bulk
}

fn parse_ts_inner(ts: &str) -> Result<TimeDelta, String> {
    let reg = checked_regex!(r#"(\W|^)(a|an|next)\W"#);
    let ts = ts.trim().to_ascii_lowercase();
    let ts = reg.replace(&ts, " 1 ");
    let ts = trim_start(&ts, ["in", "after"]).trim();

    if ts == "tomorrow" {
        return Ok(const { TimeDelta::new(60 * 60 * 24, 0).unwrap() });
    }

    let duration = humantime::parse_duration(ts).map_err(|e| match e {
        humantime::DurationError::InvalidCharacter(i) => {
            format!("non-alphanumeric character at position {}", i)
        }
        humantime::DurationError::NumberExpected(_) => {
            "did you try to write numbers as words? Use 2 instead of two".to_string()
        }
        humantime::DurationError::UnknownUnit { unit, .. } => {
            format!("unknown unit: {}", unit)
        }
        e @ (humantime::DurationError::NumberOverflow | humantime::DurationError::Empty) => {
            e.to_string()
        }
    })?;
    let delta = TimeDelta::from_std(duration).map_err(|_| "duration out of range".to_string())?;
    if delta.num_days() > 365 {
        return Err("Can't set reminder for over a year into the future".to_string());
    }
    // truncate to a minute
    // let delta = TimeDelta::new(delta.num_seconds() / 60 * 60, 0)
    //     .ok_or_else(|| "duration out of range".to_string())?;
    // if delta.num_minutes() < 1 {
    //     return Err("Can't set reminder for under a minute into the future".to_string());
    // };

    Ok(delta)
}

fn trim_start(mut s: &str, prefix: impl IntoIterator<Item = impl AsRef<str>>) -> &str {
    for p in prefix {
        s = s.strip_prefix(p.as_ref()).unwrap_or(s)
    }
    s
}
