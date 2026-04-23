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
        help = "The X-API URL"
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
        env = "CACHE_ENABLED",
        default_value = "false",
        help = "Enable caching of dashboard data"
    )]
    pub cache_enabled: bool,
}
