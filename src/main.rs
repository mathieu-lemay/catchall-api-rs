use actix_web::{middleware::Logger, web, App, HttpRequest, HttpServer, Responder, Result};
use base64::{engine::general_purpose::STANDARD as b64engine, Engine as _};
use config::{Config, ConfigError};
use log::info;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
struct Body {
    json: Option<Value>,
    raw: String,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
struct CatchallResponse {
    method: String,
    path: String,
    client: ClientInfo,
    url: UrlInfo,
    headers: HashMap<String, String>,
    query_params: HashMap<String, String>,
    body: Body,
}

async fn handler(
    req: HttpRequest,
    bytes: web::Bytes,
    query: web::Query<HashMap<String, String>>,
) -> Result<impl Responder> {
    let method = req.method();
    let path = req.path();
    let client_info = get_client(&req);
    let url_info = get_url_info(&req);
    let headers = get_headers(&req);
    let body = get_body(bytes);

    let resp = CatchallResponse {
        method: method.to_string(),
        path: path.to_string(),
        client: client_info,
        url: url_info,
        headers,
        query_params: query.0,
        body,
    };

    info!(
        "{} {}\n{}",
        method,
        path,
        serde_json::to_string_pretty(&resp).expect("Error dumping resp to json")
    );

    let resp = web::Json(resp);

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

fn get_headers(request: &HttpRequest) -> HashMap<String, String> {
    request
        .headers()
        .iter()
        .map(|(n, v)| (n.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect()
}

fn get_body(bytes: web::Bytes) -> Body {
    let json: Option<Value> = serde_json::from_slice(&bytes).ok();
    let raw = b64engine.encode(bytes);

    Body { json, raw }
}

fn configure_app(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("{path:.*}")
            .route(web::delete().to(handler))
            .route(web::get().to(handler))
            .route(web::patch().to(handler))
            .route(web::post().to(handler))
            .route(web::put().to(handler)),
    );
}

#[derive(Clone, Debug, Deserialize)]
struct AppSettings {
    host: String,
    port: u16,
    workers: usize,
}

fn get_config() -> Result<Config, ConfigError> {
    let env_source = config::Environment::with_prefix("CATCHALL_API");
    Ok(Config::builder()
        .set_default("host", "0.0.0.0")?
        .set_default("port", 8080)?
        .set_default("workers", 2)?
        .add_source(env_source)
        .build()
        .unwrap())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    log_rs::init();

    let settings: AppSettings = get_config()
        .expect("valid config")
        .try_deserialize()
        .expect("valid config");

    info!("Starting server on {}:{}", settings.host, settings.port);
    HttpServer::new(|| App::new().configure(configure_app).wrap(Logger::default()))
        .workers(settings.workers)
        .bind((settings.host, settings.port))?
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
        http::header::{ContentType, X_FORWARDED_FOR},
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
    async fn test_handler_returns_headers() {
        let app = get_test_app().await;

        let resp = test::TestRequest::get()
            .uri("/")
            .insert_header(("Content-Type", "application/json"))
            .insert_header(("X-Foo", "bar"))
            .insert_header(("Authorization", "tRoLoLol"))
            .send_request(&app)
            .await;

        assert!(resp.status().is_success());

        let body: CatchallResponse = test::read_body_json(resp).await;

        let mut expected = HashMap::new();
        expected.insert("content-type".to_string(), "application/json".to_string());
        expected.insert("authorization".to_string(), "tRoLoLol".to_string());
        expected.insert("x-foo".to_string(), "bar".to_string());

        assert_eq!(body.headers, expected);
    }

    #[actix_web::test]
    async fn test_handler_returns_query_params() {
        let app = get_test_app().await;

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

    #[actix_web::test]
    async fn test_handler_returns_json_body() {
        let app = get_test_app().await;

        let payload = "{\"foo\": \"bar\"}";

        let resp = test::TestRequest::post()
            .uri("/")
            .set_payload(payload)
            .insert_header(ContentType::json())
            .send_request(&app)
            .await;

        assert!(resp.status().is_success());

        let body: CatchallResponse = test::read_body_json(resp).await;

        let expected_json: Value = serde_json::from_str(payload).expect("Malformed json");
        let expected_raw = "eyJmb28iOiAiYmFyIn0=".to_string();

        assert_eq!(
            body.body,
            Body {
                json: Some(expected_json),
                raw: expected_raw
            }
        );
    }

    #[actix_web::test]
    async fn test_handler_returns_text_raw_body_as_base64() {
        let app = get_test_app().await;

        let resp = test::TestRequest::post()
            .uri("/")
            .set_payload("foobar")
            .send_request(&app)
            .await;

        assert!(resp.status().is_success());

        let body: CatchallResponse = test::read_body_json(resp).await;

        assert_eq!(
            body.body,
            Body {
                json: None,
                raw: "Zm9vYmFy".to_string()
            }
        );
    }

    #[actix_web::test]
    async fn test_handler_returns_binary_raw_body_as_base64() {
        let app = get_test_app().await;

        let resp = test::TestRequest::post()
            .uri("/")
            .set_payload(vec![
                35, 202, 75, 94, 123, 48, 108, 181, 224, 35, 30, 226, 172, 226, 125, 203, 201, 206,
                88, 83, 172, 201, 188, 96, 30, 244, 44, 65, 6, 199, 135, 93,
            ])
            .send_request(&app)
            .await;

        assert!(resp.status().is_success());

        let body: CatchallResponse = test::read_body_json(resp).await;

        assert_eq!(
            body.body,
            Body {
                json: None,
                raw: "I8pLXnswbLXgIx7irOJ9y8nOWFOsybxgHvQsQQbHh10=".to_string()
            }
        );
    }
}
