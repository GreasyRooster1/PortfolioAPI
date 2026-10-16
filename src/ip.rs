use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::num::ParseFloatError;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::http::header::{AcceptLanguage, HeaderValue};
use actix_web::HttpRequest;
use ipgeolocate::{Locator, Service};
use tracing::{error, info};
use tracing::log::warn;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

struct AuxIpData{
    user_agent: Option<String>,
    accept_language: Option<String>,
    client_hints: Option<String>,
    client_hint_mobile: Option<String>,
    client_hint_platform: Option<String>,
}

pub(crate) async fn track_ip(req: HttpRequest, conn:&PgPool){
    let client_ip = req
        .headers()
        .get("CF-Connecting-IP")
        .and_then(|v| v.to_str().ok());
    let host_url = req.headers().get("Referer")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("none")
        .to_string();
    let ip = match client_ip {
        Some(addr) => addr,
        None => {warn!("could not find ip");return}
    };
    let service = Service::IpApi;

    let data = match Locator::get(ip, service).await {
        Ok(ip) => ip,
        Err(error) => {error!("Error: {}", error); return},
    };

    let req_url = req.full_url().to_string();
    let aux = AuxIpData{
        user_agent:parse_header(req.headers().get("User-Agent")),
        accept_language: parse_header(req.headers().get("Accept-Language")),
        client_hints: parse_header(req.headers().get("Sec-Ch-Ua")),
        client_hint_mobile: parse_header(req.headers().get("Sec-Ch-Ua-Mobile")),
        client_hint_platform: parse_header(req.headers().get("Sec-Ch-Ua-Platform")),
    };
    info!("Geolocated ip: {} - {}, {} ({}) @ {}N {}W isp:{} from {} calling {}", data.ip, data.city, data.region, data.country, data.latitude, data.longitude,data.isp,host_url,req_url);
    match update_cache(conn,data, host_url,req_url,aux).await{
        Ok(_) => {},
        Err(error) => {error!("Error: {}", error); return},
    }
    println!("{:?}", ip);
}

fn parse_header(value: Option<&HeaderValue>) -> Option<String> {
    match value{
        None => None,
        Some(header) => {Some(header.to_str().unwrap().to_string())}
    }
}

fn parse_coord(value:String)->Option<f32>{
    match value.parse::<f32>(){
        Ok(v) => {Some(v)}
        Err(e) => {
            error!("Failed to parse coord: {}", e);
            None
        }
    }
}

pub async fn update_cache(conn:&PgPool, data:Locator, host_url:String, req_url:String, aux: AuxIpData) -> Result<(), Box<dyn Error>>{

    sqlx::query!(r#"
            INSERT INTO tracked_ips (ip,host_url,request_url,timestamp,city,region,country,latitude,longitude,isp,user_agent,accept_language,client_hints,client_hint_mobile,client_hint_platform)
            VALUES ($1, $2, $3, NOW(), $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14);
        "#,
        data.ip,
        host_url,
        req_url,
        data.city,
        data.region,
        data.country,
        parse_coord(data.latitude),
        parse_coord(data.longitude),
        data.isp,
        aux.user_agent,
        aux.accept_language,
        aux.client_hints,
        aux.client_hint_mobile,
        aux.client_hint_platform,
    )
        .execute(conn).await?;
    info!("tracked ip to database: {}", data.ip);
    Ok(())
}