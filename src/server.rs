use super::Result;
use futures_util::stream::StreamExt;
use headers::{ContentLength, Header};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, HeaderMap, Method, Request, Response, Server, StatusCode};
use log::{info, warn};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::BufWriter;
use tokio::{fs, io};
use tokio_util::io::StreamReader;

#[derive(Debug, Clone)]
pub struct FileDropper {
    listen_addr: SocketAddr,
    output: String,
    max_size: u64,
    html: String,
}

macro_rules! resp {
    ( $status: expr, $text: expr ) => {
        Response::builder()
            .status($status)
            .body($text.into())
            .unwrap()
    };
}

macro_rules! status_resp {
    ( $status: expr ) => {
        resp!(
            $status,
            format!(
                "{} {}",
                $status.as_u16(),
                $status.canonical_reason().unwrap_or("")
            )
        )
    };
}

impl FileDropper {
    pub fn new(
        listen_addr: SocketAddr,
        output: String,
        max_size: u64,
        html: String,
    ) -> FileDropper {
        FileDropper {
            listen_addr,
            output,
            max_size,
            html,
        }
    }

    async fn handle_req(
        self: Arc<Self>,
        req: Request<Body>,
    ) -> std::result::Result<Response<Body>, Infallible> {
        let (parts, req_body) = req.into_parts();

        let resp = match (&parts.method, parts.uri.path()) {
            (&Method::GET, "/") => resp!(StatusCode::OK, self.html.clone()),
            (&Method::POST, "/upload") => self.upload(req_body, parts.headers).await,
            (&Method::GET, _) => status_resp!(StatusCode::NOT_FOUND),
            _ => status_resp!(StatusCode::METHOD_NOT_ALLOWED),
        };

        info!(
            "Serving {} for {} {}",
            resp.status().as_u16(),
            parts.method,
            parts.uri.path()
        );

        Ok(resp)
    }

    fn check_content_length(
        self: Arc<Self>,
        req_headers: HeaderMap,
    ) -> std::result::Result<(), Response<Body>> {
        if !req_headers.contains_key(hyper::header::CONTENT_LENGTH) {
            return Err(status_resp!(StatusCode::LENGTH_REQUIRED));
        }

        match ContentLength::decode(&mut req_headers.get_all(hyper::header::CONTENT_LENGTH).iter())
        {
            Ok(ContentLength(length)) => {
                if length <= self.max_size {
                    Ok(())
                } else {
                    Err(status_resp!(StatusCode::PAYLOAD_TOO_LARGE))
                }
            }
            Err(_) => Err(status_resp!(StatusCode::BAD_REQUEST)),
        }
    }

    async fn upload(self: Arc<Self>, req_body: Body, req_headers: HeaderMap) -> Response<Body> {
        if let Err(error_resp) = self.clone().check_content_length(req_headers) {
            return error_resp;
        }

        self.write_file(req_body)
            .await
            .map(|_| status_resp!(StatusCode::OK))
            .unwrap_or_else(|e| {
                warn!("Error while handling request: {}", e.to_string());
                resp!(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            })
    }

    async fn write_file(self: Arc<Self>, body: Body) -> Result<()> {
        let new_path = format!("{}.tmp", &self.output);
        match fs::remove_file(&self.output).await {
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Ok(())
                } else {
                    Err(e)
                }
            }
            Ok(_) => Ok(()),
        }?;
        let file = File::create(&new_path).await?;
        let mut buf_writer = BufWriter::new(file);
        let mut stream_reader =
            StreamReader::new(body.map(|x| {
                x.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            }));
        io::copy(&mut stream_reader, &mut buf_writer).await?;
        fs::rename(&new_path, &self.output).await?;
        Ok(())
    }

    async fn serve_arc(self: Arc<Self>) -> Result<()> {
        let make_svc = {
            let file_dropper = self.clone();
            make_service_fn(move |_| {
                let file_dropper = file_dropper.clone();
                async {
                    Ok::<_, Infallible>(service_fn(move |req| {
                        FileDropper::handle_req(file_dropper.to_owned(), req)
                    }))
                }
            })
        };

        let server = Server::bind(&self.listen_addr).serve(make_svc);
        info!("Listening on http://{}", &self.listen_addr);

        server.await?;
        Ok(())
    }

    #[tokio::main]
    pub async fn serve(self) -> Result<()> {
        let file_dropper = Arc::new(self);
        file_dropper.serve_arc().await?;
        Ok(())
    }
}
