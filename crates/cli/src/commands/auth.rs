use {
    crate::{cli::AuthCommand, config, context::Ctx, ui},
    anyhow::{anyhow, Result},
    sigit_api::{account::incomplete_reason, models::AccountStatus},
};

pub async fn run(ctx: &Ctx, command: AuthCommand) -> Result<()> {
    match command {
        AuthCommand::Login { email } => login(ctx, email).await,
        AuthCommand::Logout => logout(ctx).await,
        AuthCommand::Status => status(ctx).await,
    }
}

async fn login(ctx: &Ctx, email: Option<String>) -> Result<()> {
    let email = match email {
        Some(email) => email,
        None => ui::prompt::text("Email")?,
    };
    let password = ui::prompt::password("Password")?;

    match ctx.client.sign_in(&email, &password).await? {
        AccountStatus::Ready { access_token } => {
            let path = config::write_token(ctx.environment, &access_token)?;
            ui::success(format!(
                "Signed in as {}. Token saved to {}.",
                ui::highlight(&email),
                path.display()
            ));
            Ok(())
        }
        AccountStatus::NotFound => Err(anyhow!(
            "No sigit.si account found for {email}. Sign up at https://sigit.si/auth/signup."
        )),
        AccountStatus::Incomplete { status } => Err(anyhow!("{}", incomplete_reason(status))),
    }
}

async fn logout(ctx: &Ctx) -> Result<()> {
    // A token file surviving from a previous, now-unauthenticated session
    // shouldn't block sign-out — clear it locally either way.
    if ctx.client.token().is_some() {
        let _ = ctx.client.sign_out().await;
    }
    config::clear_token(ctx.environment)?;
    ui::success("Signed out.");
    Ok(())
}

async fn status(ctx: &Ctx) -> Result<()> {
    if ctx.client.token().is_none() {
        println!("Not signed in to {}. Run `si auth login`.", ctx.environment);
        return Ok(());
    }

    match ctx.client.user().await {
        Ok(user) => ui::emit(ctx.json, &user, |user| {
            println!(
                "Signed in to {} as {} <{}>.",
                ctx.environment,
                ui::highlight(&user.username),
                user.email
            );
        }),
        Err(e) if e.is_auth_failure() => {
            config::clear_token(ctx.environment)?;
            Err(anyhow!(
                "Your session has expired. Run `si auth login` to sign in again."
            ))
        }
        Err(e) => Err(e.into()),
    }
}
