use {
    crate::{
        cli::HookCommand,
        context::Ctx,
        ui::{self, prompt},
    },
    anyhow::Result,
};

pub async fn run(ctx: &Ctx, command: HookCommand) -> Result<()> {
    ctx.require_token()?;

    match command {
        HookCommand::List { repo } => list(ctx, repo).await,
        HookCommand::Create { url, events, repo } => create(ctx, url, events, repo).await,
        HookCommand::Delete { id, repo, yes } => delete(ctx, id, repo, yes).await,
    }
}

async fn list(ctx: &Ctx, repo: Option<String>) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;
    let hooks = ctx.client.hooks(&slug.owner, &slug.name).await?;
    ui::emit(ctx.json, &hooks, |hooks| ui::render::hooks(hooks))
}

async fn create(ctx: &Ctx, url: String, events: Vec<String>, repo: Option<String>) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;
    let hook = ctx
        .client
        .create_hook(&slug.owner, &slug.name, &url, &events)
        .await?;

    ui::emit(ctx.json, &hook, |hook| {
        ui::success(format!("Created webhook #{}.", hook.id));
        if let Some(secret) = &hook.secret {
            println!(
                "  {} {}",
                ui::bold("Secret (shown once):"),
                ui::highlight(secret)
            );
        }
    })
}

async fn delete(ctx: &Ctx, id: i64, repo: Option<String>, yes: bool) -> Result<()> {
    let slug = ctx.resolve_repo(repo)?;

    if !yes && !prompt::confirm(&format!("Delete webhook #{id} on {slug}?"), false)? {
        ui::note("Cancelled.");
        return Ok(());
    }

    ctx.client.delete_hook(&slug.owner, &slug.name, id).await?;
    ui::success(format!("Deleted webhook #{id}."));
    Ok(())
}
