//! Binaire `palestre` : point d'entrée en ligne de commande.
//!
//! La seule sous-commande exigée par le sujet (README §2) est `exec --listen <adresse>`.
//! Les outils de développement (`run`, `asm`, `disasm`, `match`, `replay`) viendront s'y ajouter.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented,
        clippy::indexing_slicing
    )
)]

use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "palestre",
    version,
    about = "Machine à agents déterministe — PBC v1"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Démarre le serveur d'exécution distante (annexe K) sur l'adresse donnée.
    Exec {
        /// Adresse d'écoute, par exemple `127.0.0.1:9000`.
        #[arg(long)]
        listen: String,
    },
}

/// Le binaire ne panique pas plus que la bibliothèque : toute erreur remonte ici,
/// s'affiche sur la sortie d'erreur et devient un code de sortie non nul.
fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("palestre: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Exec { listen } => Err(format!(
            "`exec --listen {listen}` : serveur non encore implémenté (annexe K, spec {})",
            palestre::SPEC_VERSION
        )),
    }
}
