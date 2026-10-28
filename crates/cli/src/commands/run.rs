use {
    crate::{cli::RunCommand, context::Ctx, ui},
    anyhow::Result,
};

pub async fn run(ctx: &Ctx, command: RunCommand) -> Result<()> {
    ctx.require_token()?;

    match command {
        RunCommand::List => list(ctx).await,
        RunCommand::View { id } => view(ctx, id).await,
        RunCommand::Start { repo, prompt } => start(ctx, &repo, &prompt).await,
        RunCommand::Cancel { id } => cancel(ctx, id).await,
        RunCommand::Handoff { id } => handoff(ctx, id).await,
    }
}

async fn list(ctx: &Ctx) -> Result<()> {
    let runs = ctx.client.agent_runs().await?;
    ui::emit(ctx.json, &runs, |runs| ui::render::agent_runs(runs))
}

async fn view(ctx: &Ctx, id: i64) -> Result<()> {
    let run = ctx.client.agent_run(id).await?;
    ui::emit(ctx.json, &run, ui::render::agent_run_detail)
}

async fn start(ctx: &Ctx, repo: &str, prompt: &str) -> Result<()> {
    let slug = crate::git::RepoSlug::parse(repo)?;
    let run = ctx
        .client
        .create_agent_run(&slug.to_string(), prompt)
        .await?;
    ui::emit(ctx.json, &run, |run| {
        ui::success(format!(
            "Started run #{} on {}.",
            run.id,
            ui::highlight(&run.repo_full_name)
        ));
        ui::note(format!("Follow it with `si run view {}`.", run.id));
    })
}

async fn cancel(ctx: &Ctx, id: i64) -> Result<()> {
    let run = ctx.client.cancel_agent_run(id).await?;
    ui::emit(ctx.json, &run, |run| {
        ui::success(format!("Asked run #{} to stop.", run.id));
    })
}

/// The context is already JSON, so it prints the same with or without
/// `--json`.
async fn handoff(ctx: &Ctx, id: i64) -> Result<()> {
    let context = ctx.client.agent_run_handoff(id).await?;
    println!("{}", serde_json::to_string_pretty(&context)?);
    Ok(())
}
