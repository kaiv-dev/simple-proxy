use async_trait::async_trait;
use http::Uri;
use pingora::{prelude::*};
use std::{sync::{Arc, RwLock}};
use tracing::{info, Level, Span};
use tracing::span;
use uuid::Uuid;
use crate::config::RouteConfig;

pub struct HttpGateway {
    pub config: Arc<RwLock<RouteConfig>>,
}



pub struct Context {
    pub span: Arc<Span>
}

impl HttpGateway {
    pub fn default_err() -> Box<Error> {
        Box::new(Error{
            etype: ErrorType::HTTPStatus(404),
            esource: ErrorSource::Upstream,
            retry: false.into(),
            context: None,
            cause: None
        })
    }
}

#[async_trait]
impl ProxyHttp for HttpGateway
{
    type CTX = Context;

    fn new_ctx(&self) -> Self::CTX {
        let request_id = Uuid::new_v4().simple().to_string();
        let span = span!(Level::INFO, "", "id" = %format!("\x1b[90m{}\x1b[0m", request_id));
        Context{ span: Arc::new(span) }
    }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        let _s = _ctx.span.enter();
        let host = session.req_header().headers.get("host").cloned();
        let host = host
            .and_then(|v| v.to_str().map(|v| v.to_string()).ok())
            .unwrap_or_default();
        info!("Requested host: {}", host);
        let cfg = self.config.read().unwrap();
        let pq = session.req_header().uri.path_and_query();

        'a: {
        if let Some(dirs) = cfg.dir.domain.get(&host) {
            let Some(pq) = pq else {break 'a};
            for dir in dirs.iter() {
                if pq.path().starts_with(&dir.route) {
                    let mut uri = Uri::builder()
                        .authority(dir.listen.to_string())
                        .scheme("http");
                    info!("Uri: {:?}", uri);
                    uri = uri.path_and_query(pq.clone());
                    session.req_header_mut().set_uri(uri.build().unwrap());
                    return Ok(Box::new(HttpPeer::new(dir.listen, false, host)));
                }
            }
        }
        }

        if let Some(cfg) = cfg.http.get(&host) {
            let mut uri = Uri::builder()
                .authority(cfg.upstream.clone())
                .scheme(if cfg.https {"https"} else {"http"});
            let mut addr = cfg.addr.clone();
            if let Some(pq) = pq {
                let mut pq = pq.to_string();
                if let Some(allowed_ports) = &cfg.proxy_ports_from_prefix {
                    let mut it = pq.splitn(3, '/');
                    let _empty = it.next().unwrap_or("");
                    let port = it.next().unwrap_or("");
                    let rest = it.collect::<Vec<&str>>().join("/");
                    let Ok(port) = port.parse::<u16>() else { return Err(Self::default_err()) };
                    if !allowed_ports.contains(&port) { return Err(Self::default_err()) }
                    addr.set_port(port);
                    pq = format!("/{}", rest);
                }
                uri = uri.path_and_query(pq);
            } else {
                if cfg.proxy_ports_from_prefix.is_some() {
                    return Err(Self::default_err())
                }
            }
            session.req_header_mut().set_uri(uri.build().unwrap());
            return Ok(Box::new(HttpPeer::new(addr, cfg.https, host)));
        }
        Err(Self::default_err())
    }

    async fn request_filter(&self, _session: &mut Session, _ctx: &mut Self::CTX) -> pingora::Result<bool> {
        let _s = _ctx.span.enter();
        Ok(false)
    }

    async fn upstream_request_filter(
        &self,
        session: &mut Session,
        upstream_request: &mut RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()>
    where
        Self::CTX: Send + Sync,
    {
        let _s = _ctx.span.enter();
        let addr = session.client_addr().cloned().unwrap().as_inet().unwrap().ip().to_string();
        upstream_request
            .insert_header("X-Forwarded-For", addr.to_string())
            .unwrap();
        upstream_request
            .insert_header("X-Real-Ip", addr.to_string())
            .unwrap();
        info!("Headers for {addr} set!");
        Ok(())
    }

    async fn logging(
        &self,
        session: &mut Session,
        _e: Option<&pingora::Error>,
        _ctx: &mut Self::CTX,
    ) {
        let _s = _ctx.span.enter();
        let response_code = session
            .response_written()
            .map_or(0, |resp| resp.status.as_u16());
        info!(
            "{} response code: {response_code}",
            self.request_summary(session, _ctx)
        );
    }
}
