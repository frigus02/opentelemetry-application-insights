use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use http::{Request, Response};
use opentelemetry_http::{HttpClient, HttpError};
use std::pin::Pin;

/// todo
pub type StreamingBody = Pin<Box<dyn Stream<Item = Result<Vec<u8>, std::io::Error>> + Send + Sync>>;

/// todo
#[async_trait]
pub trait StreamingHttpClient: HttpClient {
    /// todo
    async fn send_streaming(
        &self,
        request: Request<StreamingBody>,
    ) -> Result<Response<Bytes>, HttpError> {
        let (parts, body) = request.into_parts();
        let new_body: Vec<Result<Vec<u8>, _>> = body.collect().await;
        let new_body_error: Result<Vec<Vec<u8>>, _> = new_body.into_iter().collect();
        let new_body_flattened = new_body_error?.into_iter().flatten().collect();
        let new_request = Request::from_parts(parts, new_body_flattened);
        self.send(new_request).await
    }
}

#[cfg(feature = "reqwest-client")]
mod reqwest {
    use super::{
        async_trait, Bytes, HttpError, Request, Response, StreamingBody, StreamingHttpClient,
    };
    use std::convert::TryInto;

    #[async_trait]
    impl StreamingHttpClient for reqwest::Client {
        async fn send_streaming(
            &self,
            request: Request<StreamingBody>,
        ) -> Result<Response<Bytes>, HttpError> {
            let (parts, body) = request.into_parts();
            let request =
                Request::from_parts(parts, reqwest::Body::wrap_stream(body)).try_into()?;
            let response = self.execute(request).await?;
            Ok(Response::builder()
                .status(response.status())
                .body(response.bytes().await?)?)
        }
    }

    #[async_trait]
    impl StreamingHttpClient for reqwest::blocking::Client {}
}

#[cfg(feature = "surf-client")]
mod surf {
    use super::{
        async_trait, Bytes, HttpError, Request, Response, StreamingBody, StreamingHttpClient,
    };
    use futures_util::TryStreamExt;

    #[async_trait]
    impl StreamingHttpClient for surf::Client {
        async fn send_streaming(
            &self,
            request: Request<StreamingBody>,
        ) -> Result<Response<Bytes>, HttpError> {
            let (parts, body) = request.into_parts();
            let method = parts.method.as_str().parse()?;
            let uri = parts.uri.to_string().parse()?;
            let body = surf::Body::from_reader(body.into_async_read(), None);

            let mut request_builder = surf::Request::builder(method, uri).body(body);
            let mut prev_name = None;
            for (new_name, value) in parts.headers.into_iter() {
                let name = new_name.or(prev_name).expect("the first time new_name should be set and from then on we always have a prev_name");
                request_builder = request_builder.header(name.as_str(), value.to_str()?);
                prev_name = Some(name);
            }

            let mut response = self.send(request_builder).await?;
            Ok(Response::builder()
                .status(response.status() as u16)
                .body(response.body_bytes().await?.into())?)
        }
    }
}
