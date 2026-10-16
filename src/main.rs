mod ip;
mod db;

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use std::{fs, thread};
use std::ops::Deref;
use std::time::Duration;
use actix_web::{error, get, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web::middleware::Logger;
use actix_cors::Cors;
use actix_files::NamedFile;
use actix_web::error::BlockingError;
use actix_web::http::header;
use actix_web::rt::time::{interval, Interval};
use actix_web::web::Json;
use firebase_rs::Firebase;
use serde_json::Value;
use tracing::{debug, error, info, trace};
use tracing::log::log;
use tracing_appender::rolling;
use tracing_subscriber::{fmt, EnvFilter};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use gcp_auth::CustomServiceAccount;
use firebase_realtime_database::Database;
use tokio_cron_scheduler::{Job, JobScheduler};
use chrono::Utc;
use cron::Schedule;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use sqlx::{Error, PgPool};
use tokio::runtime::Runtime;
use crate::db::setup_database_pool;

struct ProjectData{
    total_projects: i32,
    total_lines: i32,
}
static QCODE_DATA: LazyLock<Mutex<ProjectData>> = LazyLock::new(|| {
    Mutex::new(ProjectData{
        total_projects:0,
        total_lines:0,
    })
});

#[derive(Deserialize, Serialize, Debug)]
struct Project{
    name: Option<String>,
    desc:Option<String>,
    source:Option<String>,
    writeup:Option<String>,
    gallery:Option<String>,
    other_link_url:Option<String>,
    other_link_name:Option<String>,
    category_name:Option<String>,
    category_color:Option<String>,
    significance: Option<i32>,
    tags:sqlx::types::Json<Vec<RawTagData>>,
    private:Option<bool>,
    relevance: Option<f32>,
}
#[derive(Deserialize,Serialize)]
struct QueryResult{
    matches:Vec<Project>,
    count:i32,
    query:String,
}
#[derive(Deserialize, Serialize, Debug, sqlx::Type)]
#[sqlx(type_name = "tag_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
enum TagType{
    Language,
    Technology
}
#[derive(Deserialize,Serialize)]
struct TagData{
    name:String,
    tag_type:TagType,
    color:String,
    full_name:String,
    total_projects:Option<i64>,
}
#[derive(Deserialize,Serialize,Debug)]
struct RawTagData{
    name:Option<String>,
    tag_type:Option<TagType>,
    color:Option<String>,
    full_name:Option<String>,
}
#[derive(Deserialize,Serialize)]
struct DirectoryData{
    tags:Vec<TagData>,
    total_projects:i64,
}
struct AppState {
    db: PgPool,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if let Err(e) = dotenvy::dotenv() {
        println!("cargo:warning=Could not load .env file: {}", e);
    }

    let conn = setup_database_pool().await.expect("could not connect to db");

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
    run_daily_job();

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://localhost:5173")
            .allowed_origin("http://localhost:8080")
            .allowed_origin("https://dillonjw.com")
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allowed_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE])
            .max_age(3600);
        App::new()
            .app_data(web::Data::new(AppState { db: conn.clone() }))
            .wrap(Logger::new(r#"%{CF-Connecting-IP}i (%a) "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#))
            .wrap(cors)
            .service(version)
            .service(projects)
            .service(qcode_project_count)
            .service(qcode_line_count)
            .service(search_projects)
            .service(get_directory_data)
    })
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}

#[get("/version")]
async fn version(req: HttpRequest, data: web::Data<AppState>) -> impl Responder {
    ip::track_ip(req,&data.db).await;
    HttpResponse::Ok().body(include_str!("../VERSION"))
}

#[get("/projects")]
async fn projects(req: HttpRequest, data: web::Data<AppState>) -> Result<NamedFile, actix_web::Error> {
    let path: PathBuf = "./static/projects.json".into();
    let file = NamedFile::open_async(path).await?;
    ip::track_ip(req,&data.db).await;
    Ok(file)
}

#[get("/qcode_project_count")]
async fn qcode_project_count(req: HttpRequest, data: web::Data<AppState>) -> impl Responder {
    ip::track_ip(req,&data.db).await;
    Json(QCODE_DATA.lock().expect("could not lock project count").total_projects.clone())
}

#[get("/qcode_line_count")]
async fn qcode_line_count(req: HttpRequest, data: web::Data<AppState>) -> impl Responder {
    ip::track_ip(req,&data.db).await;
    Json(QCODE_DATA.lock().expect("could not lock project count").total_lines.clone())
}

#[get("/search_projects/{query}")]
async fn search_projects(req: HttpRequest, data: web::Data<AppState>, query:web::Path<String>) -> actix_web::Result<impl Responder> {
    ip::track_ip(req,&data.db).await;
    let mut query = query.into_inner();
    let query_result = if query=="" {
        sqlx::query_as!(Project,r#"
        SELECT
            p.name as "name?",
            p.desc,
            p.source,
            p.writeup,
            p.gallery,
            p.other_link_url,
            p.other_link_name,
            p.significance as "significance?",
            p.private as "private?",
            c.name AS "category_name?",
            c.color as "category_color?",
            COALESCE(
                json_agg(
                    json_build_object(
                        'name', t.name,
                        'full_name', t.full_name,
                        'tag_type', t.type,
                        'color', t.color
                    )
                ) FILTER (WHERE t.id IS NOT NULL),
                '[]'
            ) AS "tags!: sqlx::types::Json<Vec<RawTagData>>",
            -- Calculate a relevance score based on your 'A', 'B', 'C' weights
            ts_rank(search_vector, websearch_to_tsquery('english', $1)) AS relevance
        FROM projects p
        LEFT JOIN categories c ON c.id = p.category
        LEFT JOIN project_tags pt ON pt.project_id = p.id
        LEFT JOIN tags t ON t.id = pt.tag_id
        -- The @@ match operator ensures we use the GIN index
        WHERE search_vector @@ websearch_to_tsquery('english', $1)
        GROUP BY p.id, c.id
        ORDER BY relevance DESC, significance DESC
        LIMIT 10;
    "#,query)
            .fetch_all(&data.db)
            .await
    }else{
        sqlx::query_as!(Project,r#"
        SELECT
            p.name as "name?",
            p.desc,
            p.source,
            p.writeup,
            p.gallery,
            p.other_link_url,
            p.other_link_name,
            p.significance as "significance?",
            p.private as "private?",
            c.name AS "category_name?",
            c.color as "category_color?",
            COALESCE(
                json_agg(
                    json_build_object(
                        'name', t.name,
                        'full_name', t.full_name,
                        'tag_type', t.type,
                        'color', t.color
                    )
                ) FILTER (WHERE t.id IS NOT NULL),
                '[]'
            ) AS "tags!: sqlx::types::Json<Vec<RawTagData>>",
            0.0 as "relevance?: f32"
        FROM projects p
        LEFT JOIN categories c ON c.id = p.category
        LEFT JOIN project_tags pt ON pt.project_id = p.id
        LEFT JOIN tags t ON t.id = pt.tag_id
        GROUP BY p.id, c.id
        ORDER BY significance DESC
        LIMIT 10;
    "#)
            .fetch_all(&data.db)
            .await
    };


    let query_result = match query_result {
        Ok(query_result) => query_result,
        Err(e) => {println!("{:?}",e);return Err(e).map_err(|err| error::ErrorBadRequest(err))}
    };

    println!("{:?}", query_result);

    Ok(Json(QueryResult{
        matches: query_result,
        count: 0,
        query: query.clone(),
    }))
}

#[get("/get_directory_data")]
async fn get_directory_data(req: HttpRequest,data: web::Data<AppState>) -> impl Responder {
    ip::track_ip(req,&data.db).await;
    let tags = sqlx::query_as!(TagData,r#"
        SELECT
            t.name,
            t.color,
            t.type as "tag_type: TagType",
            t.full_name,
            COUNT(p.tag_id) AS total_projects
        FROM
            tags t
        LEFT JOIN
            project_tags p ON p.tag_id = t.id
        GROUP BY
            p.tag_id,
            t.name,
            t.color,
            t.type,
            t.full_name
    "#)
        .fetch_all(&data.db)
        .await;

    let tags = match tags {
        Ok(tags) => tags,
        Err(e) => {println!("{:?}",e);return Err(e).map_err(|err| error::ErrorBadRequest(err))}
    };

    let total_projects = sqlx::query_scalar!(r#"
        SELECT COUNT(*) FROM projects;
    "#)
        .fetch_one(&data.db)
        .await;

    let total_projects = match total_projects {
        Ok(total_projects) => total_projects,
        Err(e) => {println!("{:?}",e);return Err(e).map_err(|err| error::ErrorBadRequest(err))}
    };

    match total_projects {
        None => {
            Ok(Json(DirectoryData{
                tags,
                total_projects:0,
            }))
        }
        Some(total_projects) => {
            Ok(Json(DirectoryData{
                tags,
                total_projects,
            }))
        }
    }


}

fn run_daily_job() {
    thread::spawn(|| {
        let expression = "0 0 1 * * *";
        let schedule = Schedule::from_str(expression).expect("Failed to parse CRON expression");

        loop {
            for datetime in schedule.upcoming(Utc).take(1) {
                let now = Utc::now();
                let until = datetime - now;
                thread::sleep(until.to_std().unwrap());
                println!("Hello, world!");
                let rt = Runtime::new().unwrap();
                rt.block_on(update_qcode_project_count());
            }
        }
    });
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


    let mut proj_count = 0;
    let mut usr_count = 0;
    let mut line_count = 0;
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
                let lines = calculate_user_lines(projs);

                line_count += lines;
                proj_count += val;
                usr_count+=1;
                info!("{username} ({key}) has {val} projects and {lines} lines");
            }
            None => {
                continue;
            }
        }

    }

    info!("Total projects: {proj_count} with {line_count} from {usr_count} users");
    *QCODE_DATA.lock().unwrap() = ProjectData{
        total_projects: proj_count as i32,
        total_lines: line_count,
    }
}

fn calculate_user_lines(projs: &Value) -> i32 {
    let mut total:i32 = 0;
    for (key,data) in projs.as_object().unwrap() {
        let code = match data.get("code") {
            Some(code) => code.as_str().unwrap_or(""),
            None => continue,
        };
        let lines: Vec<&str> = code.split('\n').collect();
        total += lines.len() as i32;
    }
    total
}
