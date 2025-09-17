use std::{collections::HashMap, path::PathBuf};

use hickory_resolver::{Resolver, TokioResolver, name_server::GenericConnector, proto::runtime::TokioRuntimeProvider};
use http::{uri::Authority};
use pingora::protocols::l4::socket::SocketAddr;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use crate::wrap;



#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TcpRecord {
    pub domain: String,
    pub upstream: String
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HttpRecord {
    pub domain: String,
    pub upstream: String,
    pub routes: Option<Vec<String>>,
    pub https: Option<bool>,
    pub proxy_ports_from_prefix: Option<Vec<u16>>,
    pub strip_route: Option<bool>,
}



impl HttpParsedRecord {
    async fn try_parse(
        resolver: &Option<Resolver<GenericConnector<TokioRuntimeProvider>>>,
        upstream: String, 
        https: bool, 
        proxy_ports_from_prefix: Option<Vec<u16>>, 
        routes: Option<Vec<String>>,
        strip_route: Option<bool>,
    ) -> Option<Self> {
        let Ok(mut authority) = upstream.parse() else {
            warn!("Can't parse upstream to authority: {}, skipping", upstream);
            return None;
        };
        let maybe_addr = upstream.parse();
        let addr = match maybe_addr {
            Ok(addr) => addr,
            Err(_) => {
                let Some(resolver) = resolver else {
                    warn!("Can't parse upstream to socket: {upstream}, skipping");  
                    return None;
                };
                let Some((hostname, port)) = upstream.rsplit_once(":") else {
                    warn!("Can't parse upstream to socket: {upstream}, skipping");
                    return None;
                };
                let Ok(port) = port.parse() else {
                    warn!("Can't parse port for {upstream}");
                    return None;
                };
                info!("Resolving {}", hostname);
                let ip = match resolver.lookup_ip(hostname).await {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("Failed to resolve ip addr for {hostname}: {e}");
                        return None;
                    }
                };
                let Some(addr) = ip.iter().next() else {
                    warn!("Failed to resolve ip addr for 
                    {hostname}");
                    return None;
                };
                let addr = SocketAddr::Inet(std::net::SocketAddr::new(addr, port));
                authority = addr.to_string().parse()
                    .inspect_err(|_| warn!("Can't parse upstream to authority: {}, skipping", upstream))
                    .ok()?;
                info!("Resolved {upstream} to {addr}");
                addr
            }
        };
        
        Some(HttpParsedRecord {
            upstream: authority,
            addr,
            strip_route: strip_route.unwrap_or(false),
            routes: routes
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|r| !(r.is_empty() || r == "/"))
                        .collect(),
            https,
            proxy_ports_from_prefix, 
        })
    }
}


#[derive(Clone, Debug)]
pub struct HttpParsedRecord {
    #[allow(unused)]
    pub upstream: Authority,
    pub addr: SocketAddr,
    pub routes: Vec<String>,
    pub https: bool,
    pub strip_route: bool,
    pub proxy_ports_from_prefix: Option<Vec<u16>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DirRecord {
    pub domain: String,
    pub listen: String,
    pub path: PathBuf,
    pub route: String
}

#[derive(Clone, Debug)]
pub struct DirParsedRecord {
    // pub domain: String,
    pub listen: std::net::SocketAddr,
    pub path: PathBuf,
    pub route: String
}



#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigRecord {
    #[serde(default)]
    tcp: HashMap<String, Vec<TcpRecord>>,
    #[serde(default)]
    http: Vec<HttpRecord>,
    #[serde(default)]
    dir: Vec<DirRecord>
}

impl ConfigRecord {
    pub fn from_file(path: &str) -> anyhow::Result<ConfigRecord> {
        Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
    }

    pub async fn to_route_config(self) -> RouteConfig {
        let resolver = TokioResolver::builder(
            GenericConnector::new(TokioRuntimeProvider::default()))
                    .map(|builder| builder.build());
        let resolver = resolver.inspect_err(|e|warn!("Failed to create resolver: {e}")).ok();
        if resolver.is_some(){info!("Resolver created!");}

        let mut parsed_tcp = HashMap::new();
        for (k, v) in self.tcp {
            let Some(k) = k.parse().ok() else { warn!("Can't parse port from {k}! Skipping..."); continue };
            let mut inner = HashMap::new();
            for r in v {
                inner.insert(r.domain.clone(), r);
            }
            parsed_tcp.insert(k, inner);
        }
        let mut http_records: HashMap<String, Vec<HttpParsedRecord>> = HashMap::new();
        for record in self.http {
            let HttpRecord {
                domain, 
                upstream, 
                https, 
                proxy_ports_from_prefix,
                routes,
                strip_route,
                ..
            } = record;
            let parsed = HttpParsedRecord::try_parse(
                &resolver,
                upstream, 
                https.unwrap_or(false), 
                proxy_ports_from_prefix,
                routes,
                strip_route,
            ).await;
            let Some(parsed) = parsed else { continue };
            http_records.entry(domain).or_default().push(parsed);
        }
        RouteConfig {
            tcp: TcpConfig(parsed_tcp),
            http: HttpConfig(http_records),
            dir: DirConfig::from_record(self.dir)
        }
    }
}


#[allow(unused)]
#[derive(Default, Debug, Clone)]
pub struct RouteConfig {
    pub tcp: TcpConfig,
    pub http: HttpConfig,
    pub dir: DirConfig
}

wrap!(pub TcpConfig(pub HashMap<u16, HashMap<String, TcpRecord>>) = Default, Debug, Clone);
wrap!(pub HttpConfig(pub HashMap<String, Vec<HttpParsedRecord>>) = Default, Debug, Clone);
// wrap!(pub DirConfig(pub HashMap<String, Vec<DirParsedRecord>>) = Default, Debug, Clone);

#[derive(Default, Debug, Clone)]
pub struct DirConfig {
    pub domain: HashMap<String, Vec<DirParsedRecord>>,
    pub listen: HashMap<std::net::SocketAddr, Vec<DirParsedRecord>>
}


impl DirConfig {
    pub fn from_record(dir: Vec<DirRecord>) -> DirConfig {
        let mut domain: HashMap<String, Vec<DirParsedRecord>> = HashMap::new();
        let mut listen: HashMap<std::net::SocketAddr, Vec<DirParsedRecord>> = HashMap::new();
        for r in dir {
            let l: Result<std::net::SocketAddr, std::net::AddrParseError> = r.listen.parse();
            let Ok(listen_addr) = l else {
                warn!("Can't parse listen to socket: {}, skipping", r.listen);
                continue;
            };
            domain.entry(r.domain.clone()).or_default().push(DirParsedRecord {
                // domain: r.domain.clone(),
                listen: listen_addr.clone(),
                path: r.path.clone(),
                route: r.route.clone()
            });
            listen.entry(listen_addr).or_default().push(DirParsedRecord {
                // domain: r.domain,
                listen: listen_addr,
                path: r.path,
                route: r.route
            });
        }
        DirConfig { domain, listen }
    }
}

