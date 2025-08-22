use std::{collections::HashMap, path::PathBuf};

use http::{uri::Authority};
use pingora::protocols::l4::socket::SocketAddr;
use serde::{Deserialize, Serialize};
use tracing::warn;
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
    pub https: bool,
    pub proxy_ports_from_prefix: Option<Vec<u16>>,
}

impl HttpParsedRecord {
    fn try_parse(upstream: String, https: bool, proxy_ports_from_prefix: Option<Vec<u16>>) -> Option<Self> {
        let Ok(authority) = upstream.parse() else {
            warn!("Can't parse upstream to authority: {}, skipping", upstream);
            return None;
        };
        let Ok(addr) = upstream.parse() else {
            warn!("Can't parse upstream to socket: {}, skipping", upstream);
            return None;
        };
        Some(HttpParsedRecord {
            upstream: authority,
            addr,
            https,
            proxy_ports_from_prefix, 
        })
    }
}





#[derive(Clone, Debug)]
pub struct HttpParsedRecord {
    pub upstream: Authority,
    pub addr: SocketAddr,
    pub https: bool,
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

    pub fn to_route_config(self) -> RouteConfig {
        let mut parsed_tcp = HashMap::new();
        for (k, v) in self.tcp {
            let Some(k) = k.parse().ok() else { warn!("Can't parse port from {k}! Skipping..."); continue };
            let mut inner = HashMap::new();
            for r in v {
                inner.insert(r.domain.clone(), r);
            }
            parsed_tcp.insert(k, inner);
        }
        RouteConfig {
            tcp: TcpConfig(parsed_tcp),
            http: HttpConfig(
                self.http.into_iter().filter_map(|v| {
                    let HttpRecord {domain, upstream, https, proxy_ports_from_prefix, ..} = v;
                    Some((domain, HttpParsedRecord::try_parse(upstream, https, proxy_ports_from_prefix)?))
                }).collect()
            ),
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
wrap!(pub HttpConfig(pub HashMap<String, HttpParsedRecord>) = Default, Debug, Clone);
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

