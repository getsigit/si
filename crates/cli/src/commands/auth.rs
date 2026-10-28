use {
    crate::{
        cli::{AuthCommand, GitCredentialOperation},
        config,
        context::Ctx,
        credential, ui,
    },
    anyhow::{anyhow, Context, Result},
    sigit_si_api::{account::incomplete_reason, models::AccountStatus},
    std::io::{self, Write},
};

pub async fn run(ctx: &Ctx, command: AuthCommand) -> Result<()> {
    match command {
        AuthCommand::Login { email } => login(ctx, email).await,
        AuthCommand::Logout => logout(ctx).await,
        AuthCommand::Status => status(ctx).await,
        AuthCommand::Signup { email } => signup(ctx, email).await,
        AuthCommand::Resend { email } => resend(ctx, email).await,
        AuthCommand::ResetPassword { email } => reset_password(ctx, email).await,
        AuthCommand::DeleteAccount { yes } => delete_account(ctx, yes).await,
        AuthCommand::SetupGit => setup_git(ctx),
        AuthCommand::GitCredential { operation } => git_credential(ctx, operation).await,
    }
}

fn setup_git(ctx: &Ctx) -> Result<()> {
    let exe = std::env::current_exe().context("Could not locate the `si` executable.")?;
    let base_url = ctx.client.base_url();
    credential::setup_global_helper(base_url, &exe)?;
    ui::success(format!(
        "Git will now sign in to {} through `si`.",
        ui::highlight(base_url)
    ));
    Ok(())
}

/// Answer git's credential protocol. `store` and `erase` do nothing, since the
/// password is minted on demand and never kept. Anything this helper can't
/// answer for stays silent so git moves on to the next helper.
async fn git_credential(ctx: &Ctx, operation: GitCredentialOperation) -> Result<()> {
    if !matches!(operation, GitCredentialOperation::Get) {
        return Ok(());
    }

    let request = credential::parse_request(io::stdin().lock());
    if ctx.client.token().is_none() || !credential::is_for_host(&request, ctx.client.base_url()) {
        return Ok(());
    }

    let minted = ctx.client.git_credential().await?;
    let mut stdout = io::stdout().lock();
    stdout.write_all(credential::response(&minted.username, &minted.git_token).as_bytes())?;
    stdout.flush()?;
    Ok(())
}

fn email_or_prompt(email: Option<String>) -> Result<String> {
    match email {
        Some(email) => Ok(email),
        None => ui::prompt::text("Email"),
    }
}

async fn signup(ctx: &Ctx, email: Option<String>) -> Result<()> {
    let email = email_or_prompt(email)?;
    let password = ui::prompt::new_password("Password")?;

    let result = ctx.client.sign_up(&email, &password).await?;
    ui::emit(ctx.json, &result, |result| {
        let message = result["message"]
            .as_str()
            .unwrap_or("Signed up. Check your email to confirm your account.");
        ui::success(message);
        ui::note("Once it is confirmed, run `si auth login`.");
    })
}

async fn resend(ctx: &Ctx, email: Option<String>) -> Result<()> {
    let email = email_or_prompt(email)?;
    ctx.client.resend_confirmation(&email).await?;
    ui::success(format!(
        "If {} has an account that is not confirmed yet, a new email is on its way.",
        ui::highlight(&email)
    ));
    Ok(())
}

async fn reset_password(ctx: &Ctx, email: Option<String>) -> Result<()> {
    let email = email_or_prompt(email)?;
    ctx.client.reset_password(&email).await?;
    ui::success(format!(
        "If {} has an account, reset instructions are on their way.",
        ui::highlight(&email)
    ));
    Ok(())
}

async fn delete_account(ctx: &Ctx, yes: bool) -> Result<()> {
    ctx.require_token()?;

    if !yes {
        let user = ctx.client.user().await?;
        ui::note(format!(
            "This permanently deletes the account {} and everything in it.",
            user.email
        ));
        let typed = ui::prompt::text(&format!("Type {} to confirm", user.email))?;
        if !typed.trim().eq_ignore_ascii_case(&user.email) {
            ui::note("Cancelled.");
            return Ok(());
        }
    }

    ctx.client.delete_account().await?;
    config::clear_token(ctx.environment)?;
    ui::success("Your account was deleted.");
    Ok(())
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
