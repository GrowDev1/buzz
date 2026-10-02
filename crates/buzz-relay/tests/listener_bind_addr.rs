use std::process::Command;

use buzz_relay::config::Config;

#[test]
fn invalid_listener_addresses_fail_configuration() {
    const CHILD: &str = "BUZZ_S02_CONFIG_CHILD";
    if std::env::var_os(CHILD).is_some() {
        assert!(
            Config::from_env().is_err(),
            "invalid listener address must fail configuration before startup"
        );
        return;
    }

    for variable in ["BUZZ_HEALTH_BIND_ADDR", "BUZZ_METRICS_BIND_ADDR"] {
        for invalid in ["not-an-address", "127.0.0.999:8080", "[::gg]:8080"] {
            let output = Command::new(std::env::current_exe().expect("test executable"))
                .env_clear()
                .env(CHILD, "1")
                .env(variable, invalid)
                .args(["--exact", "invalid_listener_addresses_fail_configuration"])
                .output()
                .expect("run isolated configuration test");
            assert!(
                output.status.success(),
                "{variable}={invalid}: {}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
    }
}

#[test]
fn selected_listener_addresses_are_bound() {
    const CHILD: &str = "BUZZ_S02_BIND_CHILD";
    if let Ok(ip) = std::env::var(CHILD) {
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        runtime.block_on(async {
            let config = Config::from_env().expect("valid addresses");
            let health = config.bind_health_listener().await.expect("health bind");
            assert_eq!(health.local_addr().expect("address").ip().to_string(), ip);

            let reservation = std::net::TcpListener::bind((ip.as_str(), 0)).expect("free port");
            let addr = reservation.local_addr().expect("metrics address");
            drop(reservation);
            // A wildcard IPv4 exporter would conflict with this other loopback address.
            let _other = (ip == "127.0.0.1").then(|| {
                std::net::TcpListener::bind(("127.0.0.2", addr.port())).expect("other loopback")
            });
            buzz_relay::metrics::try_install_at(
                std::net::SocketAddr::new(config.metrics_bind_addr.ip(), addr.port()),
                300,
            )
            .expect("metrics bind");
            tokio::net::TcpStream::connect(addr)
                .await
                .expect("metrics accepts");
            assert!(std::net::TcpListener::bind(addr).is_err());
        });
        return;
    }
    for (ip, address) in [("127.0.0.1", "127.0.0.1:0"), ("::1", "[::1]:0")] {
        let output = Command::new(std::env::current_exe().expect("test executable"))
            .env_clear()
            .env(CHILD, ip)
            .env("BUZZ_HEALTH_BIND_ADDR", address)
            .env("BUZZ_METRICS_BIND_ADDR", address)
            .args(["--exact", "selected_listener_addresses_are_bound"])
            .output()
            .expect("isolated listener test");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

#[test]
fn default_listener_addresses_preserve_legacy_ports() {
    const CHILD: &str = "BUZZ_S02_DEFAULT_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let config = Config::from_env().expect("defaults");
        assert_eq!(config.health_bind_addr.to_string(), "0.0.0.0:18080");
        assert_eq!(config.metrics_bind_addr.to_string(), "0.0.0.0:19102");
        return;
    }
    let output = Command::new(std::env::current_exe().expect("test executable"))
        .env_clear()
        .env(CHILD, "1")
        .env("BUZZ_HEALTH_PORT", "18080")
        .env("BUZZ_METRICS_PORT", "19102")
        .args([
            "--exact",
            "default_listener_addresses_preserve_legacy_ports",
        ])
        .output()
        .expect("isolated default test");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}
