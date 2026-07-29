use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use actix_web::middleware::Logger;
use actix_cors::Cors;
use actix_web::http::header;
use tracing_appender::rolling;
use tracing_subscriber::{fmt, EnvFilter};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if let Err(e) = dotenvy::dotenv() {
        println!("cargo:warning=Could not load .env file: {}", e);
    }

    let file_appender = rolling::Builder::new()
        .rotation(rolling::Rotation::DAILY)
        .filename_prefix("api.log")
        .max_log_files(14)
        .build("./logs")
        .expect("failed to build appender");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        // file layer — daily rotating, no ANSI colors in files
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        // stdout layer — keep console output too
        .with(fmt::layer().with_writer(std::io::stdout))
        .init();

    HttpServer::new(|| {
        let cors = Cors::default()
            .allowed_origin("http://localhost:5173")
            .allowed_origin("https://dillonjw.com")
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allowed_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE])
            .max_age(3600);
        App::new()
            .wrap(Logger::new(r#"%{CF-Connecting-IP}i (%a) "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#))
            .wrap(cors)
            .service(version)
    })
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}

#[get("/version")]
async fn version() -> impl Responder {
    HttpResponse::Ok().body("V0.1.0")
}