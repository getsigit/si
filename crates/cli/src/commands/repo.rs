use {
    crate::{
        cli::RepoCommand,
        context::Ctx,
        ui::{self, prompt},
    },
    anyhow::Result,
    sigit_si_api::NewRepository,
};

pub async fn run(ctx: &Ctx, command: RepoCommand) -> Result<()> {
    ctx.require_token()?;

    match command {
        RepoCommand::List => list(ctx).await,
        RepoCommand::Search { query, limit } => search(ctx, query, limit).await,
        RepoCommand::View { repo } => view(ctx, repo).await,
        RepoCommand::Create {
            name,
            private,
            description,
            yes,
        } => create(ctx, name, private, description, yes).await,
        RepoCommand::Browse { repo } => browse(ctx, repo).await,
    }
}

async fn list(ctx: &Ctx) -> Result<()> {
    let repos = ctx.client.repos().await?;
    ui::emit(ctx.json, &repos, |repos| ui::render::repos(repos))
}

async fn search(ctx: &Ctx, query: Option<String>, limit: u32) -> Result<()> {
    let repos = ctx
        .client
        .search_repos(query.as_deref(), Some(limit))
        .await?;
    ui::emit(ctx.json, &repos, |repos| ui::render::repo_summaries(repos))
}

async fn view(ctx: &Ctx, repo: Option<String>) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;

    // `GET /api/v1/repos` only lists the signed-in user's own repos, but
    // `view` should also work on repos owned by someone else — so try the
    // owned list first (it has the fuller `Repository` shape) and fall back
    // to the cross-user MCP search.
    if let Some(repo) = ctx
        .client
        .repos()
        .await?
        .into_iter()
        .find(|r| r.full_name.eq_ignore_ascii_case(&slug.to_string()))
    {
        return ui::emit(ctx.json, &repo, ui::render::repo_detail);
    }

    let matches = ctx.client.search_repos(Some(&slug.name), None).await?;
    let summary = matches
        .into_iter()
        .find(|r| r.full_name.eq_ignore_ascii_case(&slug.to_string()))
        .ok_or_else(|| anyhow::anyhow!("Repository {slug} was not found."))?;
    ui::emit(ctx.json, &summary, |summary| {
        ui::render::repo_summaries(std::slice::from_ref(summary))
    })
}

async fn create(
    ctx: &Ctx,
    name: String,
    private: bool,
    description: Option<String>,
    yes: bool,
) -> Result<()> {
    if !yes {
        let confirmed = prompt::confirm(
            &format!(
                "Create {} repository {}?",
                if private { "private" } else { "public" },
                ui::highlight(&name)
            ),
            true,
        )?;
        if !confirmed {
            ui::note("Cancelled.");
            return Ok(());
        }
    }

    let new_repo = NewRepository::new(&name)
        .private(private)
        .description(description);
    let repo = ctx.client.create_repo(&new_repo).await?;

    ui::emit(ctx.json, &repo, |repo| {
        ui::success(format!("Created {}.", ui::highlight(&repo.full_name)));
        println!("  {}", repo.clone_url);
    })
}

async fn browse(ctx: &Ctx, repo: Option<String>) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;
    let url = format!("{}/{slug}", ctx.client.base_url());
    open::that(&url)?;
    ui::note(format!("Opened {url}"));
    Ok(())
}
