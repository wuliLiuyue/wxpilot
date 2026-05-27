use wxp_license_server::{serve, ServerConfig};

fn main() {
    if let Err(error) = ServerConfig::from_env().and_then(serve) {
        eprintln!("ERR: {error}");
        std::process::exit(1);
    }
}
