use clap::Parser;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Config {
    #[clap(
        long,
        env = "LISTEN",
        default_value = "0.0.0.0:3000",
        help = "The address to listen on"
    )]
    pub listen: String,

    #[clap(
        long,
        env = "ONKOSTAR_URL",
        default_value = "http://localhost:8080/onkostar",
        help = "The X-API URL"
    )]
    pub onkostar_url: String,
}
