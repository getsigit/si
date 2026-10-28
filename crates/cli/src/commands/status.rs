use {
    crate::{cli::StatusCommand, context::Ctx, git, ui},
    anyhow::{Context, Result},
    sigit_si_api::NewStatus,
};

pub async fn run(ctx: &Ctx, command: StatusCommand) -> Result<()> {
    ctx.require_token()?;

    match command {
        StatusCommand::Set {
            sha,
            state,
            context,
            description,
            url,
            repo,
        } => set(ctx, sha, state, context, description, url, repo).await,
        StatusCommand::View { sha, repo } => view(ctx, sha, repo).await,
    }
}

fn resolve(rev: &str) -> Result<String> {
    let cwd = std::env::current_dir().context("Could not read the current directory.")?;
    git::resolve_sha(&cwd, rev)
}

async fn set(
    ctx: &Ctx,
    rev: String,
    state: String,
    context: Option<String>,
    description: Option<String>,
    url: Option<String>,
    repo: Option<String>,
) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;
    let sha = resolve(&rev)?;

    let status = NewStatus::new(state)
        .context(context)
        .description(description)
        .target_url(url);
    let created = ctx
        .client
        .create_status(&slug.owner, &slug.name, &sha, &status)
        .await?;

    ui::emit(ctx.json, &created, |created| {
        ui::success(format!(
            "Reported {} for {} on {}.",
            created.state,
            ui::highlight(&created.context),
            &created.sha[..created.sha.len().min(12)]
        ));
    })
}

async fn view(ctx: &Ctx, rev: Option<String>, repo: Option<String>) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;
    let sha = resolve(rev.as_deref().unwrap_or("HEAD"))?;

    let combined = ctx
        .client
        .combined_status(&slug.owner, &slug.name, &sha)
        .await?;
    ui::emit(ctx.json, &combined, ui::render::combined_status)
}
