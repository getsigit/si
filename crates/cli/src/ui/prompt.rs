//! Interactive prompts, kept in one place so every command fails the same way
//! when stdin isn't a TTY (CI, pipes) instead of hanging.

use {
    anyhow::{anyhow, Result},
    console::Term,
    dialoguer::{theme::ColorfulTheme, Confirm, Input, Password},
};

fn require_tty(what: &str) -> Result<()> {
    if Term::stdout().features().is_attended() {
        Ok(())
    } else {
        Err(anyhow!(
            "{what} requires an interactive terminal. Provide it via a flag or environment \
             variable instead when running non-interactively."
        ))
    }
}

pub fn text(label: &str) -> Result<String> {
    require_tty(label)?;
    Ok(Input::with_theme(&ColorfulTheme::default())
        .with_prompt(label)
        .interact_text()?)
}

pub fn password(label: &str) -> Result<String> {
    require_tty(label)?;
    Ok(Password::with_theme(&ColorfulTheme::default())
        .with_prompt(label)
        .interact()?)
}

/// A yes/no confirmation. Non-interactive callers should not reach this at
/// all for destructive actions — check `--yes` first — but reads default to
/// `default` here rather than erroring, since a TTY-less "confirm the details
/// I already showed you" is reasonable to skip.
pub fn confirm(label: &str, default: bool) -> Result<bool> {
    if !Term::stdout().features().is_attended() {
        return Ok(default);
    }
    Ok(Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(label)
        .default(default)
        .interact()?)
}
