use {
    crate::{cli::PrCommand, context::Ctx, git, ui},
    anyhow::{anyhow, Result},
    sigit_api::State,
};

pub async fn run(ctx: &Ctx, command: PrCommand) -> Result<()> {
    ctx.require_token()?;

    match command {
        PrCommand::List { repo, state } => list(ctx, repo, state).await,
        PrCommand::View { number, repo } => view(ctx, number, repo).await,
        PrCommand::Diff { number, repo } => diff(ctx, number, repo).await,
        PrCommand::Create {
            title,
            head,
            base,
            body,
            repo,
        } => create(ctx, title, head, base, body, repo).await,
        PrCommand::Comment { number, body, repo } => comment(ctx, number, body, repo).await,
    }
}

async fn list(ctx: &Ctx, repo: Option<String>, state: String) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;
    let state = parse_state(&state)?;
    let pulls = ctx.client.pulls(&slug.to_string(), state).await?;
    ui::emit(ctx.json, &pulls, |pulls| ui::render::pulls(pulls))
}

async fn view(ctx: &Ctx, number: i64, repo: Option<String>) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;
    let pr = ctx.client.pull(&slug.to_string(), number).await?;
    ui::emit(ctx.json, &pr, ui::render::pull_detail)
}

async fn diff(ctx: &Ctx, number: i64, repo: Option<String>) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;
    let pr = ctx.client.pull(&slug.to_string(), number).await?;

    match pr.diff {
        Some(diff) => {
            print!("{diff}");
            if let Some(note) = pr.diff_note {
                ui::note(note);
            }
        }
        None => {
            ui::note(
                pr.diff_note
                    .unwrap_or_else(|| "No diff available.".to_string()),
            );
        }
    }
    Ok(())
}

async fn create(
    ctx: &Ctx,
    title: String,
    head: Option<String>,
    base: String,
    body: Option<String>,
    repo: Option<String>,
) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;
    let head = match head {
        Some(head) => head,
        None => git::current_branch(&std::env::current_dir()?)
            .ok_or_else(|| anyhow!("Could not determine the current branch. Pass --head."))?,
    };

    let created = ctx
        .client
        .create_pull(&slug.to_string(), &title, &head, &base, body.as_deref())
        .await?;

    ui::emit(ctx.json, &created, |created| {
        ui::success(format!("Opened pull request #{}.", created.number));
        println!("  {}", created.url);
    })
}

async fn comment(ctx: &Ctx, number: i64, body: String, repo: Option<String>) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;
    let comment = ctx
        .client
        .add_comment(&slug.to_string(), number, &body)
        .await?;
    ui::emit(ctx.json, &comment, |comment| {
        ui::success(format!("Commented on #{number}."));
        println!("  {}", comment.url);
    })
}

fn parse_state(input: &str) -> Result<State> {
    match input.to_ascii_lowercase().as_str() {
        "open" => Ok(State::Open),
        "closed" => Ok(State::Closed),
        "merged" => Ok(State::Merged),
        "all" => Ok(State::All),
        other => Err(anyhow!(
            "Unknown state {other:?}. Expected one of: open, closed, merged, all."
        )),
    }
}
