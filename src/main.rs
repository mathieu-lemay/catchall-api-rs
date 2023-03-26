use actix_web::{middleware, web, App, HttpRequest, HttpServer, Responder, Result};
use env_logger::Env;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
struct CatchallResponse {
    method: String,
    path: String,
    query_params: HashMap<String, String>,
}

async fn handler(
    req: HttpRequest,
    query: web::Query<HashMap<String, String>>,
) -> Result<impl Responder> {
    let resp = CatchallResponse {
        method: req.method().to_string(),
        path: req.path().to_string(),
        query_params: query.0,
    };

    let resp = web::Json(resp);

    println!("{:?}", resp);

    Ok(resp)
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
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::header::ContentType, test};

    #[actix_web::test]
    async fn test_handler_empty_request() {
        let app = test::init_service(App::new().configure(configure_app)).await;
        let req = test::TestRequest::default()
            .insert_header(ContentType::plaintext())
            .to_request();

        let resp: CatchallResponse = test::call_and_read_body_json(&app, req).await;
        let expected = CatchallResponse {
            method: "GET".to_string(),
            path: "/".to_string(),
            ..Default::default()
        };

        assert_eq!(resp, expected);
    }

    #[actix_web::test]
    async fn test_handler_returns_path() {
        let app = test::init_service(App::new().configure(configure_app)).await;
        let req = test::TestRequest::get().uri("/foo/bar?baz=1").to_request();

        let resp: CatchallResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp.path, "/foo/bar".to_string());
    }

    #[actix_web::test]
    async fn test_handler_returns_method() {
        let app = test::init_service(App::new().configure(configure_app)).await;

        let req = test::TestRequest::delete().uri("/").to_request();
        let resp: CatchallResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp.method, "DELETE".to_string());

        let req = test::TestRequest::get().uri("/").to_request();
        let resp: CatchallResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp.method, "GET".to_string());

        let req = test::TestRequest::patch().uri("/").to_request();
        let resp: CatchallResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp.method, "PATCH".to_string());

        let req = test::TestRequest::post().uri("/").to_request();
        let resp: CatchallResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp.method, "POST".to_string());

        let req = test::TestRequest::put().uri("/").to_request();
        let resp: CatchallResponse = test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp.method, "PUT".to_string());
    }

    #[actix_web::test]
    async fn test_handler_returns_query_params() {
        let app = test::init_service(App::new().configure(configure_app)).await;

        let req = test::TestRequest::get()
            .uri("/?foo=bar&baz=69")
            .to_request();
        let resp: CatchallResponse = test::call_and_read_body_json(&app, req).await;

        let mut expected = HashMap::new();
        expected.insert("foo".to_string(), "bar".to_string());
        expected.insert("baz".to_string(), "69".to_string());

        assert_eq!(resp.query_params, expected);
    }
}
