use clap::Parser;

#[derive(Parser, Default)]
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
        help = "The Onkostar base URL"
    )]
    pub onkostar_url: String,

    #[clap(
        long,
        env = "COOKIE_DOMAIN",
        help = "The cookie domain to be used (optional)"
    )]
    pub cookie_domain: Option<String>,

    #[clap(
        long,
        env = "CACHE_DURATION",
        help = "Enable response caching with the given duration (optional)"
    )]
    pub cache_duration: Option<humantime::Duration>,
}
