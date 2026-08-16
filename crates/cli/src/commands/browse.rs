//! `si browse [target]` — open something on sigit.si in the default browser.
//!
//! `target` accepts a bare `OWNER/NAME`, `OWNER/NAME/pull/N`, or
//! `OWNER/NAME/issue/N`; omitted, it falls back to the repository in the
//! current directory.

use {
    crate::{cli::BrowseArgs, context::Ctx, ui},
    anyhow::{anyhow, Result},
};

pub async fn run(ctx: &Ctx, args: BrowseArgs) -> Result<()> {
    let url = match args.target {
        Some(target) => resolve_target(ctx, &target)?,
        None => {
            let slug = ctx.resolve_repo(None)?;
            format!("{}/{slug}", ctx.client.base_url())
        }
    };

    open::that(&url)?;
    ui::note(format!("Opened {url}"));
    Ok(())
}

/// `OWNER/NAME`, `OWNER/NAME/pull/N`, or `OWNER/NAME/issue/N`.
fn resolve_target(ctx: &Ctx, target: &str) -> Result<String> {
    let trimmed = target.trim().trim_matches('/');
    let parts: Vec<&str> = trimmed.split('/').collect();

    match parts.as_slice() {
        [owner, name] => Ok(format!("{}/{owner}/{name}", ctx.client.base_url())),
        [owner, name, kind @ ("pull" | "issue"), number] => {
            number
                .parse::<u64>()
                .map_err(|_| anyhow!("Expected a number after {kind}/, got {number:?}."))?;
            Ok(format!(
                "{}/{owner}/{name}/{kind}/{number}",
                ctx.client.base_url()
            ))
        }
        _ => Err(anyhow!(
            "Could not parse {target:?}. Expected OWNER/NAME, OWNER/NAME/pull/N, or \
             OWNER/NAME/issue/N."
        )),
    }
}
