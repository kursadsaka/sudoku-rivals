#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = listen_address(
        std::env::var("SUDOKU_BIND").ok().as_deref(),
        std::env::var("PORT").ok().as_deref(),
        std::env::var("SUDOKU_PORT").ok().as_deref(),
    )?;
    let mut config = sudoku_server::http::Config::default();
    if let Some(root) = std::env::var_os("SUDOKU_WEB_ROOT") {
        config.web_root = root.into();
    }
    config.public_ingress = !address.ip().is_loopback();
    config.validate_assets()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    let app = sudoku_server::http::router_with_config(sudoku_server::shared_service(), config);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;
    Ok(())
}

fn listen_address(
    bind: Option<&str>,
    port: Option<&str>,
    local_port: Option<&str>,
) -> Result<std::net::SocketAddr, Box<dyn std::error::Error>> {
    let ip = bind.unwrap_or("127.0.0.1").parse::<std::net::IpAddr>()?;
    let port = port.or(local_port).unwrap_or("3000").parse::<u16>()?;
    if port == 0 {
        return Err("PORT/SUDOKU_PORT must be between 1 and 65535".into());
    }
    Ok(std::net::SocketAddr::new(ip, port))
}

#[cfg(test)]
mod tests {
    use super::listen_address;

    #[test]
    fn render_port_requires_explicit_public_bind_and_takes_precedence() {
        assert_eq!(
            listen_address(None, None, None).unwrap().to_string(),
            "127.0.0.1:3000"
        );
        assert_eq!(
            listen_address(None, Some("10000"), None)
                .unwrap()
                .to_string(),
            "127.0.0.1:10000"
        );
        assert_eq!(
            listen_address(Some("0.0.0.0"), Some("10000"), Some("3001"))
                .unwrap()
                .to_string(),
            "0.0.0.0:10000"
        );
        assert_eq!(
            listen_address(None, None, Some("3001")).unwrap().port(),
            3001
        );
        for (bind, port) in [
            (Some("example.com"), Some("3000")),
            (None, Some("0")),
            (None, Some("65536")),
            (None, Some("")),
        ] {
            assert!(listen_address(bind, port, Some("3001")).is_err());
        }
    }
}
