use {
    crate::{cli::IssueCommand, context::Ctx, ui},
    anyhow::{anyhow, Result},
    sigit_si_api::State,
};

pub async fn run(ctx: &Ctx, command: IssueCommand) -> Result<()> {
    ctx.require_token()?;

    match command {
        IssueCommand::List { repo, state, query } => list(ctx, repo, state, query).await,
        IssueCommand::View { number, repo } => view(ctx, number, repo).await,
        IssueCommand::Create { title, body, repo } => create(ctx, title, body, repo).await,
        IssueCommand::Comment { number, body, repo } => comment(ctx, number, body, repo).await,
    }
}

async fn list(ctx: &Ctx, repo: Option<String>, state: String, query: Option<String>) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;
    let state = parse_state(&state)?;
    let issues = ctx
        .client
        .issues(&slug.to_string(), state, query.as_deref())
        .await?;
    ui::emit(ctx.json, &issues, |issues| ui::render::issues(issues))
}

async fn view(ctx: &Ctx, number: i64, repo: Option<String>) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;
    let issue = ctx.client.issue(&slug.to_string(), number).await?;
    ui::emit(ctx.json, &issue, ui::render::issue_detail)
}

async fn create(
    ctx: &Ctx,
    title: String,
    body: Option<String>,
    repo: Option<String>,
) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;
    let created = ctx
        .client
        .create_issue(&slug.to_string(), &title, body.as_deref())
        .await?;

    ui::emit(ctx.json, &created, |created| {
        ui::success(format!("Opened issue #{}.", created.number));
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
        "all" => Ok(State::All),
        other => Err(anyhow!(
            "Unknown state {other:?}. Expected one of: open, closed, all."
        )),
    }
}
