use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub image: String,

    #[arg(short, long)]
    pub mount_point: String,
}
