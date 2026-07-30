use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use std::{fs, thread};
use std::time::Duration;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use actix_web::middleware::Logger;
use actix_cors::Cors;
use actix_files::NamedFile;
use actix_web::error::BlockingError;
use actix_web::http::header;
use actix_web::rt::time::{interval, Interval};
use actix_web::web::Json;
use firebase_rs::Firebase;
use serde_json::Value;
use tracing::{debug, error, info};
use tracing::log::log;
use tracing_appender::rolling;
use tracing_subscriber::{fmt, EnvFilter};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use gcp_auth::CustomServiceAccount;
use firebase_realtime_database::Database;


static PROJECT_COUNT: LazyLock<Mutex<i32>> = LazyLock::new(|| {
    Mutex::new(0)
});

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
    update_qcode_project_count().await;

    thread::spawn(||{
       run_daily_job()
    });


    HttpServer::new(|| {
        let cors = Cors::default()
            .allowed_origin("http://localhost:5173")
            .allowed_origin("http://localhost:8080")
            .allowed_origin("https://dillonjw.com")
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allowed_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE])
            .max_age(3600);
        App::new()
            .wrap(Logger::new(r#"%{CF-Connecting-IP}i (%a) "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#))
            .wrap(cors)
            .service(version)
            .service(projects)
            .service(qcode_project_count)
    })
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}

#[get("/version")]
async fn version() -> impl Responder {
    HttpResponse::Ok().body("V0.1.0")
}

#[get("/projects")]
async fn projects() -> Result<NamedFile, actix_web::Error> {
    let path: PathBuf = "./static/projects.json".into();
    let file = NamedFile::open_async(path).await?;
    Ok(file)
}

#[get("/qcode_project_count")]
async fn qcode_project_count() -> impl Responder {
    Json(PROJECT_COUNT.lock().expect("could not lock project count").clone())
}

async fn run_daily_job() {
    let mut daily_interval: Interval = interval(Duration::from_secs(24 * 60 * 60));

    loop {
        daily_interval.tick().await;
        info!("Running daily task...");
        update_qcode_project_count().await;
    }
}


async fn update_qcode_project_count(){
    let database_ref = Database::from_path("qcode-cdfc6-default-rtdb", "./service-account.json");
    let db = match database_ref {
        Ok(d) => d,
        Err(err) => {error!("Could not connect to firebase: {err}"); return;},
    };
    let data = match db.get("userdata").await {
        Ok(res) => {res.json::<Value>().await.expect("could not parse request")}
        Err(err) => {error!("userdata request failed"); return;},
    };


    let mut count = 0;
    let mut usr_count = 0;
    for (key,data) in data.as_object().unwrap() {
        let projs_option = data.get("projects");
        let username = match data.get("username") {
            Some(u) => u.as_str().unwrap_or("(no username)"),
            None => "(no username)",
        };
        info!("Parsing projects for: {} ({key})", username);
        match projs_option{
            Some(projs) => {
                let val = projs.as_object().unwrap().len();
                count += val;
                usr_count+=1;
                info!("{username} ({key}) has {val} projects");
            }
            None => {
                continue;
            }
        }

    }

    info!("Total projects: {count} from {usr_count} users");
    *PROJECT_COUNT.lock().unwrap() = count as i32;
}
