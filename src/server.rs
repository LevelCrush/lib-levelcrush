use crate::util::unix_timestamp;
use axum::{error_handling::HandleErrorLayer, http::StatusCode, response::IntoResponse, BoxError, Json, Router};
use std::{net::SocketAddr, time::Duration};
use tower::{buffer::BufferLayer, limit::RateLimitLayer, ServiceBuilder};
use {
    axum::http::{HeaderValue, Method},
    tower_http::cors::{AllowOrigin, CorsLayer},
};
use {
    axum_sessions::async_session::MemoryStore,
    axum_sessions::{SameSite, SessionLayer},
};

#[derive(serde::Serialize, serde::Deserialize, Default, Debug, Clone)]
pub struct PaginationData {
    pub total_results: u32,
    pub total_pages: u32,
    pub page: u32,
    pub limit: u32,
    pub showing: usize,
    pub term: String,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug, Clone)]
pub struct PaginationResponse<T: serde::Serialize> {
    pub data: Vec<T>,
    pub pagination: PaginationData,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug, Clone)]
pub struct APIResponseError {
    pub field: String,
    pub message: String,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug, Clone)]
pub struct APIResponse<T: serde::ser::Serialize> {
    success: bool,
    response: Option<T>,
    errors: Vec<APIResponseError>,
    requested_at: i64,
    completed_at: i64,
}

impl<T: serde::ser::Serialize> APIResponse<T> {
    /// generate a new resposne
    pub fn new() -> APIResponse<T> {
        APIResponse {
            success: false,
            response: None,
            errors: Vec::new(),
            requested_at: unix_timestamp(),
            completed_at: 0,
        }
    }

    /// sets the data for the request
    pub fn data(&mut self, response: Option<T>) {
        self.response = response;
    }

    /// pushes an error onto the response.
    pub fn error<FieldName: Into<String>, Message: Into<String>>(&mut self, field: FieldName, message: Message) {
        self.errors.push(APIResponseError {
            field: field.into(),
            message: message.into(),
        });
    }

    /// marks the request as completed and checks if the request was a success
    pub fn complete(&mut self) {
        self.success = self.response.is_some() && self.errors.is_empty();
        self.completed_at = unix_timestamp();
    }
}

async fn setup_session_layer(secret: String) -> SessionLayer<MemoryStore> {
    let store = MemoryStore::new();
    SessionLayer::new(store, secret.as_bytes())
        .with_secure(true)
        .with_same_site_policy(SameSite::None)
}

pub struct Server {
    port: u16,
    allow_session: bool,
    allow_cors: bool,
    rate_limit: bool,
    rate_limit_num: u64,
    rate_limit_per: Duration,
    rate_limit_buffer: u64,
    session_secret: Option<String>,
}

impl Server {
    pub fn new(port: u16) -> Server {
        Server {
            port,
            allow_session: false,
            allow_cors: false,
            rate_limit: false,
            rate_limit_num: 1,
            rate_limit_per: Duration::from_secs(60),
            rate_limit_buffer: 1024,
            session_secret: None,
        }
    }

    pub fn enable_rate_limit(mut self, num: u64, per: Duration, backpressure: u64) -> Self {
        self.rate_limit = true;
        self.rate_limit_num = num;
        self.rate_limit_per = per;
        self.rate_limit_buffer = backpressure;
        self
    }

    /// turn on the cors layer
    pub fn enable_cors(mut self) -> Self {
        self.allow_cors = true;
        self
    }

    /// turn on the session layer
    pub fn enable_session(mut self, secret: &str) -> Self {
        self.allow_session = true;
        self.session_secret = Some(secret.to_string());
        self
    }

    pub async fn build<AppState, F>(self, routes: Router<AppState>, pre_session_layer: F, state: AppState) -> Router
    where
        AppState: Send + Sync + Clone + 'static,
        F: FnOnce(Router) -> Router,
    {
        // configure our router
        #[allow(unused_mut)]
        let mut router = Router::new().nest("/", routes).fallback(unknown_path).with_state(state);

        // turn on cors
        if self.allow_cors {
            tracing::info!("Here before cors");

            tracing::info!("Enabling cors!");
            // cors layer ( for our services, this is just about always going to be the same)
            // we may allow this to be extended in the future but for now it works
            let origins = vec![
                HeaderValue::from_str("https://levelcrush.local").unwrap(),
                HeaderValue::from_str("https://preview.levelcrush.com").unwrap(),
                HeaderValue::from_str("https://www.levelcrush.com").unwrap(),
                HeaderValue::from_str("https://levelcrush.com").unwrap(),
            ];

            // enable cors
            let cors_layer = CorsLayer::new()
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                .allow_origin(AllowOrigin::from(origins))
                .allow_credentials(true);

            router = router.layer(cors_layer);
        }

        if self.rate_limit {
            router = router.layer(
                ServiceBuilder::new()
                    .layer(HandleErrorLayer::new(|err: BoxError| async move {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("An internal error has occurred {}", err),
                        )
                    }))
                    .layer(BufferLayer::new(self.rate_limit_buffer as usize))
                    .layer(RateLimitLayer::new(self.rate_limit_num, self.rate_limit_per)),
            );
        }

        if self.allow_session {
            tracing::info!("Enabling In-Memory session!");
            let session_layer = setup_session_layer(self.session_secret.unwrap_or_default()).await;
            router = router.layer(session_layer);
        }

        // run the callback
        router = pre_session_layer(router);

        router
    }

    /// construct and run the router service and then run it
    pub async fn run_custom<AppState, F>(self, routes: Router<AppState>, custom: F, state: AppState)
    where
        AppState: Send + Sync + Clone + 'static,
        F: FnOnce(Router) -> Router,
    {
        let port = self.port;
        let mut router = self.build(routes, |r| r, state).await;

        router = custom(router);

        // run the server
        // construct and output what interface/port we are intending on using
        Server::run_router(router, port).await;
    }

    pub async fn run<AppState>(self, routes: Router<AppState>, state: AppState)
    where
        AppState: Send + Sync + Clone + 'static,
    {
        let port = self.port;
        let router = self.build(routes, |r| r, state).await;
        Server::run_router(router, port).await;
    }

    pub async fn run_router(router: Router, port: u16) {
        // run the server
        // construct and output what interface/port we are intending on using
        let server_addr = SocketAddr::from(([0, 0, 0, 0], port));
        tracing::info!("listening on {}", server_addr);
        let _ = axum::Server::bind(&server_addr).serve(router.into_make_service()).await;
    }
}

async fn unknown_path() -> impl IntoResponse {
    let mut api_response = APIResponse::<Option<String>>::new();

    api_response.error("url", "Not found");

    api_response.complete();
    (StatusCode::NOT_FOUND, Json(api_response).into_response())
}
