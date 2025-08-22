use axum::{
    Router,
};
use tracing::warn;
use tower_http::{
    services::{ServeDir},
    trace::TraceLayer,
};

use crate::config::DirParsedRecord;


pub fn dirs_router(dirs: Vec<DirParsedRecord>) -> Router {
    let mut app = Router::new();
    let mut root_inited = false;
    for dir in dirs {
        if dir.route == "/" {
            if root_inited {
                warn!("Root already inited, no multiple roots listeners allowed! Skipping {:?} ...", dir.path);
                continue;
            }
            app = app
                .fallback_service(ServeDir::new(dir.path));
            root_inited = true;
        } else {
            app = app
                .nest_service(&dir.route, ServeDir::new(dir.path));
        }
    }
    app
        .layer(TraceLayer::new_for_http())
}
