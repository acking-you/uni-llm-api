//! Middleware for cors

use axum::{
    body::Body,
    http::{header, HeaderValue, Method, Request, Response, StatusCode},
};
use futures::future;
use std::{
    convert::Infallible,
    task::{Context, Poll},
};
use tower::{Layer, Service};

/// A middleware for identifying CORS requests and setting the appropriate response headers correctly
#[derive(Clone)]
pub struct CorsMiddleware<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for CorsMiddleware<S>
where
    S: Service<Request<Body>, Response = Response<Body>, Error = Infallible>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let origin = req.headers().get(header::ORIGIN).cloned();
        let is_options = req.method() == Method::OPTIONS;

        let mut cloned_inner = self.inner.clone();

        Box::pin(async move {
            // Handle preflight requests
            #[allow(clippy::unnecessary_unwrap)]
            if is_options && origin.is_some() {
                return Ok(Response::builder()
                    .status(StatusCode::NO_CONTENT)
                    .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin.expect("Origin is checked by `origin.is_some()`"))
                    .header(
                        header::ACCESS_CONTROL_ALLOW_METHODS,
                        "GET,POST,PUT,PATCH,DELETE,HEAD,OPTIONS"
                    )
                    .header(
                        header::ACCESS_CONTROL_ALLOW_HEADERS,
                        "Authorization,Content-Type,User-Agent,Accept,X-Requested-With,X-Stainless-Lang,X-Stainless-Package-Version,X-Stainless-Os,X-Stainless-Arch,X-Stainless-Retry-Count,X-Stainless-Runtime,X-Stainless-Runtime-Version,X-Stainless-Async,X-Stainless-Helper-Method,X-Stainless-Poll-Helper,X-Stainless-Custom-Poll-Interval",
                    )
                    .header(header::ACCESS_CONTROL_MAX_AGE, "43200")
                    .header(header::VARY, "Origin")
                    .header(header::VARY, "Access-Control-Request-Method")
                    .header(header::VARY, "Access-Control-Request-Headers")
                    .body(Body::empty())
                    .expect("Construct response nerver fails"));
            }

            // Handling general requests
            let mut response = cloned_inner.call(req).await?;

            if let Some(origin) = origin {
                response
                    .headers_mut()
                    .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
                response.headers_mut().insert(
                    header::ACCESS_CONTROL_EXPOSE_HEADERS,
                    HeaderValue::from_static("Content-Length, X-Custom-Header"),
                );
            }

            Ok(response)
        })
    }
}

/// Layer Implementation for [`CorsMiddleware`]
#[derive(Clone)]
pub struct CorsLayer;

impl<S> Layer<S> for CorsLayer {
    type Service = CorsMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CorsMiddleware { inner }
    }
}
