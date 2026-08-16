//! Terminal presentation: symbols, colour, and the plain/JSON output switch.

pub mod prompt;
pub mod render;

use {
    chrono::{DateTime, Utc},
    console::style,
    serde::Serialize,
};

pub fn success_symbol() -> String {
    style("✔").green().to_string()
}

/// Print a completed action.
pub fn success(message: impl AsRef<str>) {
    println!("{} {}", success_symbol(), message.as_ref());
}

/// Print a non-fatal note. Goes to stderr so it never pollutes piped output.
pub fn note(message: impl AsRef<str>) {
    eprintln!("{}", style(message.as_ref()).dim());
}

pub fn highlight(text: &str) -> String {
    style(text).green().bold().to_string()
}

pub fn dim(text: &str) -> String {
    style(text).dim().to_string()
}

pub fn bold(text: &str) -> String {
    style(text).bold().to_string()
}

/// Colour an issue/PR state the way the web UI does.
pub fn state_badge(state: &str) -> String {
    match state {
        "open" => style("open").green().to_string(),
        "merged" => style("merged").magenta().to_string(),
        "closed" => style("closed").red().to_string(),
        other => style(other).dim().to_string(),
    }
}

/// Render a value as either pretty JSON or human output.
///
/// Every read command routes its output through here, so `--json` is uniformly
/// available for scripting without each command reimplementing it.
pub fn emit<T: Serialize>(json: bool, value: &T, render: impl FnOnce(&T)) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        render(value);
    }
    Ok(())
}

/// A compact "how long ago" for list views, e.g. `3d`, `about 5h`, `just now`.
pub fn relative_time(timestamp: DateTime<Utc>) -> String {
    let elapsed = Utc::now().signed_duration_since(timestamp);
    let seconds = elapsed.num_seconds();

    // A clock skew between client and server can put a timestamp slightly in
    // the future; read those as "now" rather than printing a negative age.
    if seconds < 60 {
        return "just now".to_string();
    }

    let minutes = elapsed.num_minutes();
    if minutes < 60 {
        return format!("{minutes}m ago");
    }
    let hours = elapsed.num_hours();
    if hours < 24 {
        return format!("{hours}h ago");
    }
    let days = elapsed.num_days();
    if days < 30 {
        return format!("{days}d ago");
    }
    if days < 365 {
        return format!("{}mo ago", days / 30);
    }
    format!("{}y ago", days / 365)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn recent_timestamps_read_as_just_now() {
        assert_eq!(relative_time(Utc::now()), "just now");
    }

    #[test]
    fn future_timestamps_do_not_produce_negative_ages() {
        // Clock skew between the client and server is real; never print "-3m".
        let future = Utc::now() + Duration::minutes(5);
        assert_eq!(relative_time(future), "just now");
    }

    #[test]
    fn ages_step_through_the_expected_units() {
        let now = Utc::now();
        assert_eq!(relative_time(now - Duration::minutes(5)), "5m ago");
        assert_eq!(relative_time(now - Duration::hours(5)), "5h ago");
        assert_eq!(relative_time(now - Duration::days(5)), "5d ago");
        assert_eq!(relative_time(now - Duration::days(60)), "2mo ago");
        assert_eq!(relative_time(now - Duration::days(800)), "2y ago");
    }
}
