use {
    crate::{context::Ctx, ui},
    anyhow::Result,
};

pub async fn run(ctx: &Ctx) -> Result<()> {
    ctx.require_token()?;
    let user = ctx.client.user().await?;
    ui::emit(ctx.json, &user, ui::render::user)
}
