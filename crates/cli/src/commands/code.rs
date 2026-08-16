use {
    crate::{cli::CodeCommand, context::Ctx, ui},
    anyhow::Result,
};

pub async fn run(ctx: &Ctx, command: CodeCommand) -> Result<()> {
    ctx.require_token()?;

    match command {
        CodeCommand::View { path, r#ref, repo } => view(ctx, path, r#ref, repo).await,
        CodeCommand::Search {
            query,
            r#ref,
            limit,
            repo,
        } => search(ctx, query, r#ref, limit, repo).await,
    }
}

async fn view(
    ctx: &Ctx,
    path: String,
    git_ref: Option<String>,
    repo: Option<String>,
) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;
    let file = ctx
        .client
        .file_contents(&slug.to_string(), &path, git_ref.as_deref())
        .await?;

    if ctx.json {
        println!("{}", serde_json::to_string_pretty(&file)?);
    } else {
        print!("{}", file.content);
    }
    Ok(())
}

async fn search(
    ctx: &Ctx,
    query: String,
    git_ref: Option<String>,
    limit: u32,
    repo: Option<String>,
) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;
    let hits = ctx
        .client
        .search_code(&slug.to_string(), &query, git_ref.as_deref(), Some(limit))
        .await?;
    ui::emit(ctx.json, &hits, |hits| ui::render::code_hits(hits))
}
