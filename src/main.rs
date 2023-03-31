use actix_web::{middleware, web, App, HttpRequest, HttpServer, Responder, Result};
use env_logger::Env;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
struct ClientInfo {
    remote_ip: Option<String>,
    port: u16,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
struct UrlInfo {
    scheme: String,
    hostname: String,
    port: u16,
    path: String,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
struct CatchallResponse {
    method: String,
    path: String,
    client: ClientInfo,
    url: UrlInfo,
    query_params: HashMap<String, String>,
}

async fn handler(
    req: HttpRequest,
    query: web::Query<HashMap<String, String>>,
) -> Result<impl Responder> {
    let client_info = get_client(&req);
    let url_info = get_url_info(&req);

    let resp = CatchallResponse {
        method: req.method().to_string(),
        path: req.path().to_string(),
        client: client_info,
        url: url_info,
        query_params: query.0,
    };

    let resp = web::Json(resp);

    println!("{:?}", resp);

    Ok(resp)
}

fn get_client(request: &HttpRequest) -> ClientInfo {
    let conn_info = request.connection_info();
    let remote_ip = conn_info.realip_remote_addr().map(|s| s.to_string());

    // Will only return None when called in unit tests unless TestRequest::peer_addr is used.
    let port = request
        .peer_addr()
        .unwrap_or_else(|| SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080))
        .port();

    ClientInfo { remote_ip, port }
}

fn get_url_info(request: &HttpRequest) -> UrlInfo {
    let conn_info = request.connection_info();

    let host = conn_info.host();
    let hostname = host.split(':').next().unwrap_or("").to_string();
    let port = host
        .split(':')
        .nth(1)
        .map(|p| p.parse::<u16>().unwrap_or(0))
        .unwrap_or(0);

    UrlInfo {
        scheme: conn_info.scheme().to_string(),
        hostname,
        port,
        path: request.path().to_string(),
    }
}

fn configure_app(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("{path:.*}")
            .route(web::delete().to(handler))
            .route(web::get().to(handler))
            .route(web::patch().to(handler))
            .route(web::post().to(handler))
            .route(web::put().to(handler))
            .wrap(middleware::Logger::default()),
    );
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    HttpServer::new(|| App::new().configure(configure_app))
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_http::Request;
    use actix_web::{
        body::BoxBody,
        dev::{Service, ServiceResponse},
        http::header::X_FORWARDED_FOR,
        test,
    };

    async fn get_test_app(
    ) -> impl Service<Request, Response = ServiceResponse<BoxBody>, Error = actix_web::Error> {
        test::init_service(App::new().configure(configure_app)).await
    }

    #[actix_web::test]
    async fn test_handler_empty_request() {
        let app = get_test_app().await;

        let resp = test::TestRequest::get()
            .uri("/")
            .peer_addr("192.168.42.69:12345".parse().unwrap())
            .send_request(&app)
            .await;

        assert!(resp.status().is_success());

        let body: CatchallResponse = test::read_body_json(resp).await;
        let expected = CatchallResponse {
            method: "GET".to_string(),
            path: "/".to_string(),
            client: ClientInfo {
                remote_ip: Some("192.168.42.69".to_string()),
                port: 12345,
            },
            url: UrlInfo {
                scheme: "http".to_string(),
                hostname: "localhost".to_string(),
                port: 8080,
                path: "/".to_string(),
            },
            ..Default::default()
        };

        assert_eq!(body, expected);
    }

    #[actix_web::test]
    async fn test_handler_returns_path() {
        let app = get_test_app().await;
        let resp = test::TestRequest::get()
            .uri("/foo/bar?baz=1")
            .send_request(&app)
            .await;

        assert!(resp.status().is_success());

        let body: CatchallResponse = test::read_body_json(resp).await;

        assert_eq!(body.path, "/foo/bar".to_string());
    }

    #[actix_web::test]
    async fn test_handler_returns_method() {
        let app = get_test_app().await;

        let resp = test::TestRequest::delete()
            .uri("/")
            .send_request(&app)
            .await;
        let body: CatchallResponse = test::read_body_json(resp).await;
        assert_eq!(body.method, "DELETE".to_string());

        let resp = test::TestRequest::patch().uri("/").send_request(&app).await;
        let body: CatchallResponse = test::read_body_json(resp).await;
        assert_eq!(body.method, "PATCH".to_string());

        let resp = test::TestRequest::post().uri("/").send_request(&app).await;
        let body: CatchallResponse = test::read_body_json(resp).await;
        assert_eq!(body.method, "POST".to_string());

        let resp = test::TestRequest::post().uri("/").send_request(&app).await;
        let body: CatchallResponse = test::read_body_json(resp).await;
        assert_eq!(body.method, "POST".to_string());

        let resp = test::TestRequest::put().uri("/").send_request(&app).await;
        let body: CatchallResponse = test::read_body_json(resp).await;
        assert_eq!(body.method, "PUT".to_string());
    }

    #[actix_web::test]
    async fn test_handler_returns_real_ip() {
        let app = get_test_app().await;

        let ip = "192.168.0.1";

        let resp = test::TestRequest::get()
            .uri("/")
            .insert_header((X_FORWARDED_FOR, ip))
            .peer_addr("127.0.0.1:12345".parse().unwrap())
            .send_request(&app)
            .await;

        assert!(resp.status().is_success());

        let body: CatchallResponse = test::read_body_json(resp).await;

        assert_eq!(body.client.remote_ip, Some(ip.to_string()));
    }

    #[actix_web::test]
    async fn test_handler_returns_query_params() {
        let app = test::init_service(App::new().configure(configure_app)).await;

        let resp = test::TestRequest::get()
            .uri("/?foo=bar&baz=69")
            .send_request(&app)
            .await;

        assert!(resp.status().is_success());

        let body: CatchallResponse = test::read_body_json(resp).await;

        let mut expected = HashMap::new();
        expected.insert("foo".to_string(), "bar".to_string());
        expected.insert("baz".to_string(), "69".to_string());

        assert_eq!(body.query_params, expected);
    }
}
