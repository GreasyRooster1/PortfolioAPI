use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use actix_web::middleware::Logger;
use actix_cors::Cors;
use actix_web::http::header;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    if let Err(e) = dotenvy::dotenv() {
        println!("cargo:warning=Could not load .env file: {}", e);
    }

    HttpServer::new(|| {
        let cors = Cors::default()
            .allowed_origin("http://localhost:5173")
            .allowed_origin("https://dillonjw.com")
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allowed_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE])
            .max_age(3600);
        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .service(version)
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}

#[get("/version")]
async fn version() -> impl Responder {
    HttpResponse::Ok().body("V0.1.0")
}