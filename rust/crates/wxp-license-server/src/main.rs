use wxp_license_server::{serve, ServerConfig};

fn main() {
    if let Err(error) = serve(ServerConfig::from_env()) {
        eprintln!("ERR: {error}");
        std::process::exit(1);
    }
}
