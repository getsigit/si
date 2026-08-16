use {
    crate::{
        cli::SessionCommand,
        context::Ctx,
        ui::{self, prompt},
    },
    anyhow::Result,
};

pub async fn run(ctx: &Ctx, command: SessionCommand) -> Result<()> {
    ctx.require_token()?;

    match command {
        SessionCommand::List => list(ctx).await,
        SessionCommand::View { id } => view(ctx, id).await,
        SessionCommand::New { title, model } => new(ctx, title, model).await,
        SessionCommand::Update { id, title, model } => update(ctx, id, title, model).await,
        SessionCommand::Delete { id, yes } => delete(ctx, id, yes).await,
    }
}

async fn list(ctx: &Ctx) -> Result<()> {
    let sessions = ctx.client.sessions().await?;
    ui::emit(ctx.json, &sessions, |sessions| {
        ui::render::sessions(sessions)
    })
}

async fn view(ctx: &Ctx, id: i64) -> Result<()> {
    let session = ctx.client.session(id).await?;
    ui::emit(ctx.json, &session, ui::render::session_detail)
}

async fn new(ctx: &Ctx, title: Option<String>, model: Option<String>) -> Result<()> {
    let session = ctx
        .client
        .create_session(title.as_deref(), model.as_deref())
        .await?;
    ui::emit(ctx.json, &session, |session| {
        ui::success(format!("Started session #{}.", session.id));
    })
}

async fn update(ctx: &Ctx, id: i64, title: Option<String>, model: Option<String>) -> Result<()> {
    let session = ctx
        .client
        .update_session(id, title.as_deref(), model.as_deref())
        .await?;
    ui::emit(ctx.json, &session, |session| {
        ui::success(format!("Updated session #{}.", session.id));
    })
}

async fn delete(ctx: &Ctx, id: i64, yes: bool) -> Result<()> {
    if !yes && !prompt::confirm(&format!("Delete session #{id}?"), false)? {
        ui::note("Cancelled.");
        return Ok(());
    }

    ctx.client.delete_session(id).await?;
    ui::success(format!("Deleted session #{id}."));
    Ok(())
}
