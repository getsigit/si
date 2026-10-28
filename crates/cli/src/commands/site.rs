use {
    crate::{
        cli::SiteCommand,
        context::Ctx,
        ui::{self, prompt},
    },
    anyhow::{anyhow, Result},
    sigit_si_api::models::Site,
    std::time::Duration,
};

/// How often `publish --wait` asks, and how many times: about 90 seconds.
const POLL_INTERVAL: Duration = Duration::from_secs(2);
const POLL_ATTEMPTS: u32 = 45;

pub async fn run(ctx: &Ctx, command: SiteCommand) -> Result<()> {
    ctx.require_token()?;

    match command {
        SiteCommand::View { repo } => view(ctx, repo).await,
        SiteCommand::Publish { wait, repo } => publish(ctx, wait, repo).await,
        SiteCommand::Unpublish { repo, yes } => unpublish(ctx, repo, yes).await,
    }
}

async fn view(ctx: &Ctx, repo: Option<String>) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;
    let site = ctx.client.site(&slug.owner, &slug.name).await?;
    ui::emit(ctx.json, &site, ui::render::site)
}

async fn publish(ctx: &Ctx, wait: bool, repo: Option<String>) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;
    let mut site = ctx.client.publish_site(&slug.owner, &slug.name).await?;

    if wait {
        ui::note("Publishing…");
        for _ in 0..POLL_ATTEMPTS {
            if is_settled(&site) {
                break;
            }
            tokio::time::sleep(POLL_INTERVAL).await;
            site = ctx.client.site(&slug.owner, &slug.name).await?;
        }
        if !is_settled(&site) {
            return Err(anyhow!(
                "The site is still publishing. Check on it with `si site view`."
            ));
        }
    }

    let failed = site.state == "failed";
    ui::emit(ctx.json, &site, |site| {
        if failed {
            ui::render::site(site);
        } else if site.state == "live" {
            ui::success(format!("Published {}.", ui::highlight(&site.url)));
        } else {
            ui::success(format!("Publishing {}.", ui::highlight(&site.url)));
            ui::note("Check on it with `si site view`.");
        }
    })?;

    if failed {
        return Err(anyhow!(
            "Publishing failed: {}",
            site.error.as_deref().unwrap_or("no reason given.")
        ));
    }
    Ok(())
}

async fn unpublish(ctx: &Ctx, repo: Option<String>, yes: bool) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;
    if !yes && !prompt::confirm(&format!("Take the site for {slug} offline?"), false)? {
        ui::note("Cancelled.");
        return Ok(());
    }

    ctx.client.unpublish_site(&slug.owner, &slug.name).await?;
    ui::success(format!("Taking the site for {slug} offline."));
    Ok(())
}

fn is_settled(site: &Site) -> bool {
    matches!(site.state.as_str(), "live" | "failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(state: &str) -> Site {
        serde_json::from_value(serde_json::json!({
            "url": "https://demo.example",
            "state": state,
            "error": null,
            "last_published_at": null,
            "files": 0,
            "bytes": 0,
        }))
        .unwrap()
    }

    #[test]
    fn live_and_failed_end_the_wait() {
        assert!(is_settled(&site("live")));
        assert!(is_settled(&site("failed")));
    }

    #[test]
    fn publishing_keeps_the_wait_going() {
        assert!(!is_settled(&site("publishing")));
    }
}
