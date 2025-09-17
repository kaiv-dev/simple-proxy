use pingora::{ prelude::*, server::configuration::ServerConf, server::Server};
use pingora::listeners::tls::TlsSettings;
use rustls::crypto::ring::default_provider;
use rustls::crypto::CryptoProvider;
use std::{path::PathBuf, sync::Arc};
use tracing::{info, warn};

use crate::config::{ConfigRecord, RouteConfig};
use crate::services::http::HttpGateway;
use crate::services::dir::dirs_router;

mod config;
mod services;
mod util;


env_config!(
    ".cfg" => CFG = Cfg {
        CERT_PATH: String = "./certs".to_string(),
        CONFIG_PATH: String = "./proxy.toml".to_string(),
        LISTEN_ADDR: String = "0.0.0.0:443".to_string(),
        HTTPS : bool = true,
        GRACE_PERIOD: u64 = u64::MAX,
        GRACEFUL_SHUTDOWN_TIMEOUT: u64 = u64::MAX
    }
);


fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    CryptoProvider::install_default(default_provider()).ok();

    let mut server = Server::new_with_opt_and_conf(None, ServerConf{
        grace_period_seconds: Some(CFG.GRACE_PERIOD),
        graceful_shutdown_timeout_seconds: Some(CFG.GRACEFUL_SHUTDOWN_TIMEOUT),
        ..Default::default()
    });

    server.bootstrap();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let config = runtime.block_on(async {
        let config = load_config(&CFG.CONFIG_PATH).await.expect("Can't load config");
        let shared_config: Arc<RouteConfig> = Arc::new(config.clone());
        for (host, dirs) in config.dir.listen.into_iter() {
            let r = dirs_router(dirs);
            tokio::spawn(async move {
                let l = tokio::net::TcpListener::bind(host).await;
                let Ok(listener) = l else {
                    warn!("Can't bind port for dir server: {}", l.unwrap_err());
                    return;
                };
                info!("Starting dir server on {}", host);
                if let Err(e) = axum::serve(listener, r).await 
                {
                    tracing::error!("Dir server on {} failed: {}", host, e);
                }
            });
        }
        shared_config
    });

    let mut proxy = http_proxy_service(&server.configuration, HttpGateway{config: Arc::clone(&config)});
    let cert_path = format!("{}/fullchain.pem", CFG.CERT_PATH);
    let key_path = format!("{}/privkey.pem", CFG.CERT_PATH);

    if CFG.HTTPS {
        if PathBuf::from(&cert_path).exists() && PathBuf::from(&key_path).exists() {
            let tls = TlsSettings::intermediate(&cert_path, &key_path)?;
            proxy.add_tls_with_settings(&CFG.LISTEN_ADDR, None, tls);
        } else {
            warn!("Can't find cert or key");
        }
    } else {
        proxy.add_tcp(&CFG.LISTEN_ADDR);
    }
    info!("Proxy listening on {} {} tls encryption", CFG.LISTEN_ADDR, if CFG.HTTPS { "with" } else { "without" });

    server.add_service(proxy);
    server.run_forever();
}

async fn load_config(path: &str) -> anyhow::Result<RouteConfig> {
    Ok(ConfigRecord::from_file(path)?.to_route_config().await)
}
