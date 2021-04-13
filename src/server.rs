use super::Result;
use futures_util::stream::StreamExt;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::BufWriter;
use tokio::{fs, io};
use tokio_util::io::StreamReader;
use log::{info, warn};

#[derive(Debug, Clone)]
pub struct FileDropper {
    listen_addr: SocketAddr,
    output: String,
}

impl FileDropper {
    pub fn new(listen_addr: SocketAddr, output: String) -> FileDropper {
        FileDropper {
            listen_addr,
            output,
        }
    }

    async fn handle_req(
        self: Arc<Self>,
        req: Request<Body>,
    ) -> std::result::Result<Response<Body>, Infallible> {
        let (parts, req_body) = req.into_parts();

        let (status, resp_body) = match (&parts.method, parts.uri.path()) {
            (&Method::GET, "/") => (StatusCode::OK, String::from("Ok")),
            (&Method::POST, "/") => self
                .write_file(req_body)
                .await
                .map(|_| (StatusCode::OK, String::from("Ok")))
                .unwrap_or_else(|e| {
                    warn!("Error while handling request: {}", e.to_string());
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                }),
            (&Method::GET, _) => (StatusCode::NOT_FOUND, String::from("404 not found")),
            _ => (
                StatusCode::METHOD_NOT_ALLOWED,
                String::from("405 method not allowed"),
            ),
        };

        info!("Serving {} for {} {}", status, parts.method, parts.uri.path());

        Ok(Response::builder()
            .status(status)
            .body(resp_body.into())
            .unwrap())
    }

    async fn write_file(self: Arc<Self>, body: Body) -> Result<()> {
        let new_path = format!("{}.tmp", &self.output);
        fs::remove_file(&self.output).await?;
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
