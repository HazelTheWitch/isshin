use std::{
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::PathBuf,
};

use clap::{Parser, Subcommand};
use directories::BaseDirs;
use isshin::{user::Credentials, web::auth::RegisterResponse};
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Login {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        password: String,
        host: String,
    },
    Register {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        password: String,
    },
}

#[derive(Serialize, Deserialize)]
struct Configuration {
    #[serde(flatten)]
    credentials: Credentials,
    host: String,
}

fn get_config_directory() -> anyhow::Result<PathBuf> {
    let base = BaseDirs::new().ok_or(anyhow::anyhow!("could not get config directory"))?;

    let dir = base.config_dir().join("isshin");

    fs::create_dir_all(&dir)?;

    Ok(dir)
}

fn get_config() -> anyhow::Result<Configuration> {
    let path = get_config_directory()?.join("config.json");

    Ok(serde_json::from_reader(BufReader::new(File::open(path)?))?)
}

fn set_config(config: &Configuration) -> anyhow::Result<()> {
    let path = get_config_directory()?.join("config.json");

    serde_json::to_writer(BufWriter::new(File::create(path)?), config)?;

    Ok(())
}

async fn authenticate(client: &Client, config: &Configuration) -> anyhow::Result<()> {
    client
        .post(format!("{}/login", config.host))
        .form(&config.credentials)
        .send()
        .await?;

    Ok(())
}

async fn login(client: Client, config: &Configuration) -> anyhow::Result<()> {
    authenticate(&client, &config).await?;

    set_config(config)
}

async fn register(client: Client, credentials: Credentials) -> anyhow::Result<()> {
    let config = get_config()?;

    authenticate(&client, &config).await?;

    let response = client
        .post(format!("{}/register", &config.host))
        .form(&credentials)
        .send()
        .await?
        .json::<RegisterResponse>()
        .await?;

    match response {
        RegisterResponse::Ok => {
            println!("User successfully registered.");
            println!("Username: {}", credentials.username);
            println!("Password: {}", credentials.password);
        },
        RegisterResponse::UsernameTaken => {
            println!("Username is already taken: {}", credentials.username)
        },
        RegisterResponse::BadPassword { zxcvbn } => println!("Password too weak: {zxcvbn} < 3"),
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let arguments = Arguments::parse();

    let client = ClientBuilder::new().cookie_store(true).build()?;

    match arguments.command {
        Command::Login {
            username,
            password,
            host,
        } => {
            login(
                client,
                &Configuration {
                    host,
                    credentials: Credentials { username, password },
                },
            )
            .await
        }
        Command::Register { username, password } => {
            register(client, Credentials { username, password }).await
        }
    }
}
