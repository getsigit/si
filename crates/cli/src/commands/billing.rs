use {
    crate::{cli::BillingCommand, context::Ctx, ui},
    anyhow::Result,
};

pub async fn run(ctx: &Ctx, command: BillingCommand) -> Result<()> {
    ctx.require_token()?;

    match command {
        BillingCommand::Show => show(ctx).await,
        BillingCommand::Checkout { plan } => checkout(ctx, plan).await,
        BillingCommand::Portal => portal(ctx).await,
    }
}

async fn show(ctx: &Ctx) -> Result<()> {
    let billing = ctx.client.billing().await?;
    ui::emit(ctx.json, &billing, ui::render::billing)
}

async fn checkout(ctx: &Ctx, plan: String) -> Result<()> {
    let url = ctx.client.billing_checkout(&plan).await?;
    open_and_report(ctx, &url)
}

async fn portal(ctx: &Ctx) -> Result<()> {
    let url = ctx.client.billing_portal().await?;
    open_and_report(ctx, &url)
}

fn open_and_report(ctx: &Ctx, url: &str) -> Result<()> {
    if ctx.json {
        println!("{}", serde_json::json!({ "url": url }));
        return Ok(());
    }
    open::that(url)?;
    ui::note(format!("Opened {url}"));
    Ok(())
}
