mod healthcheck;
mod invites;

use clap::{Parser, Subcommand};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(subcommand)]
    HealthCheck(HealthCheckCommands),

    #[command(subcommand)]
    Invites(InviteCommands),
}

#[derive(Debug, Subcommand)]
enum HealthCheckCommands {
    MongoDB {}
}

#[derive(Debug, Subcommand)]
enum InviteCommands {
    List(ListInvitesArgs),
    Get(GetInvitesArgs),
    Create(CreateInviteArgs),
    Delete(DeleteInviteArgs)
}

#[derive(Debug, Parser)]
struct ListInvitesArgs {
    #[clap(long, short, action, default_missing_value="true", default_value_t=false)]
    unused_only: bool
}

#[derive(Debug, Parser)]
struct GetInvitesArgs {
    invite_code: String
}

#[derive(Debug, Parser)]
struct CreateInviteArgs {
    invite_code: Option<String>
}

#[derive(Debug, Parser)]
struct DeleteInviteArgs {
    invite_code: String
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args = Cli::parse();

    match args.command {
        Commands::HealthCheck(healthcheck) => match healthcheck {
            HealthCheckCommands::MongoDB {} => {
                healthcheck::do_mongo_check().await
            }
        },
        Commands::Invites(invites) => match invites {
            InviteCommands::List(list_invites) => {
                invites::list_invites(list_invites.unused_only).await
            },
            InviteCommands::Get(get_invites) => {
                invites::get_invite(get_invites.invite_code).await
            },
            InviteCommands::Create(create_invite) => {
                invites::create_invite(create_invite.invite_code).await
            },
            InviteCommands::Delete(delete_invite) => {
                invites::delete_invite(delete_invite.invite_code).await
            }
        },
    }
}