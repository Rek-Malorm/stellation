use futures::Future;
use warp::path::FullPath;
use warp::reject::not_found;
use warp::reply::Response;
use warp::{Filter, Rejection};

use crate::frontend::IndexHtml;
use crate::html;
use crate::request::{WarpRenderRequest, WarpRequest};

/// A filter that extracts the warp request.
pub(crate) fn warp_request() -> impl Clone
       + Send
       + Filter<
    Extract = (WarpRequest<()>,),
    Error = Rejection,
    Future = impl Future<Output = Result<(WarpRequest<()>,), Rejection>>,
> {
    warp::path::full()
        .and(warp::query::raw().or_else(|_| async move { Ok::<_, Rejection>((String::new(),)) }))
        .and(warp::header::headers_cloned())
        .then(
            move |path: FullPath, raw_queries: String, headers| async move {
                let http_headers = convert_to_http_header_map(headers);
                WarpRequest {
                    path: path.into(),
                    raw_queries: raw_queries.into(),
                    context: ().into(),
                    headers: http_headers, // Use the converted headers here
                }
            },
        )
}

/// A filter that extracts the warp render request.
pub(crate) fn warp_render_request(
    index_html: IndexHtml,
    auto_refresh: bool,
) -> impl Clone
       + Send
       + Filter<
    Extract = (WarpRenderRequest<()>,),
    Error = Rejection,
    Future = impl Future<Output = Result<(WarpRenderRequest<()>,), Rejection>>,
> {
    warp_request().then(move |req: WarpRequest<()>| {
        let index_html = index_html.clone();
        async move {
            let mut template = index_html.read_content().await;

            if auto_refresh {
                template = html::add_refresh_script(&template).into();
            }

            WarpRenderRequest {
                inner: req,
                template,
                is_client_only: false,
            }
        }
    })
}

/// A filter that rejects all responses.
pub(crate) fn reject() -> impl Clone
       + Send
       + Filter<
    Extract = (Response,),
    Error = Rejection,
    Future = impl Future<Output = Result<(Response,), Rejection>>,
> {
    warp::any().and_then(|| async move { Err::<Response, Rejection>(not_found()) })
}

fn convert_to_http_header_map(warp_headers: warp::http::HeaderMap) -> http::HeaderMap {
    let mut http_headers = http::HeaderMap::new();
    for (name, value) in warp_headers.iter() {
        if let Ok(header_name) = http::header::HeaderName::from_bytes(name.as_str().as_bytes()) {
            http_headers.insert(header_name, http::header::HeaderValue::from_bytes(value.clone().as_bytes()).unwrap());
        }
    }
    http_headers
}
