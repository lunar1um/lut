use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "lut")]
#[command(about="a horrible and broken git clone", long_about=None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Add {
        #[arg(required = true)]
        path: String,
    },
    Commit,
    Init,
    Log,
    Debug {
        #[arg(required = true)]
        hash: String,
    },
}
