use {
    crate::{context::Ctx, ui},
    anyhow::Result,
};

/// The model list is public, so this works without signing in.
pub async fn run(ctx: &Ctx) -> Result<()> {
    let models = ctx.client.models().await?;
    ui::emit(ctx.json, &models, |models| ui::render::models(models))
}
