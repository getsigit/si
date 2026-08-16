//! `si api <path>` — the escape hatch for any endpoint without a dedicated
//! command, e.g. one the server added since this build of `si`.

use {
    crate::{cli::ApiArgs, context::Ctx},
    anyhow::{anyhow, Result},
    reqwest::Method,
    std::str::FromStr,
};

pub async fn run(ctx: &Ctx, args: ApiArgs) -> Result<()> {
    ctx.require_token()?;

    let method = Method::from_str(&args.method.to_ascii_uppercase())
        .map_err(|_| anyhow!("Unknown HTTP method: {}", args.method))?;

    let body = match args.body {
        Some(raw) => {
            Some(serde_json::from_str(&raw).map_err(|e| anyhow!("--body is not valid JSON: {e}"))?)
        }
        None => None,
    };

    let response = ctx.client.raw(method, &args.path, body).await?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
