use actix_web::{get, middleware::Logger, web, web::Json, App, HttpServer};
use monitoring_core::metrics;
use psutil::cpu::CpuPercentCollector;
use std::sync::Mutex;

mod config;
mod extractor;
mod state;

use config::{read_config_toml, Config, CONFIG_FN};
use extractor::Client;
use state::CollectorState;

#[get("/")]
async fn get_all(
    _client: Client,
    collector: web::Data<CollectorState>,
) -> actix_web::Result<Json<metrics::Metrics>> {
    let cpu_metrics = collector.get_cpu_metrics();
    let memory_metrics = collector.get_memory_metrics();
    let metrics = metrics::Metrics {
        cpu: cpu_metrics,
        memory: memory_metrics,
    };
    Ok(Json(metrics))
}

#[get("/")]
async fn get_cpu(
    _client: Client,
    collector: web::Data<CollectorState>,
) -> actix_web::Result<Json<metrics::CpuMetrics>> {
    let cpu_metrics = collector.get_cpu_metrics();
    Ok(Json(cpu_metrics))
}

#[get("/")]
async fn get_cpu_load(
    _client: Client,
    collector: web::Data<CollectorState>,
) -> actix_web::Result<Json<metrics::CpuLoadMetric>> {
    let load = collector.get_cpu_metrics().load.unwrap();
    Ok(Json(load))
}

#[get("/average")]
async fn get_cpu_load_average(
    _client: Client,
    collector: web::Data<CollectorState>,
) -> actix_web::Result<Json<monitoring_core::Percent>> {
    let load = collector.get_cpu_metrics().load.unwrap();
    Ok(Json(load.average))
}

#[get("/per-core")]
async fn get_cpu_load_per_core(
    _client: Client,
    collector: web::Data<CollectorState>,
) -> actix_web::Result<Json<Vec<monitoring_core::Percent>>> {
    let load = collector.get_cpu_metrics().load.unwrap();
    Ok(Json(load.per_core.unwrap()))
}

#[get("/")]
async fn get_memory(
    _client: Client,
    collector: web::Data<CollectorState>,
) -> actix_web::Result<Json<metrics::MemoryMetrics>> {
    let memory_metrics = collector.get_memory_metrics();
    Ok(Json(memory_metrics))
}

#[get("/perc-used")]
async fn get_memory_perc_used(
    _client: Client,
    collector: web::Data<CollectorState>,
) -> actix_web::Result<Json<monitoring_core::Percent>> {
    let memory_metrics = collector.get_memory_metrics();
    Ok(Json(memory_metrics.perc_used))
}

#[get("/detailed")]
async fn get_memory_detailed(
    _client: Client,
    collector: web::Data<CollectorState>,
) -> actix_web::Result<Json<metrics::MemoryDetailedMetrics>> {
    let memory_metrics = collector.get_memory_metrics();
    Ok(Json(memory_metrics.detailed.unwrap()))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    // Init metric collector
    let collector = web::Data::new(CollectorState {
        cpu_collector: Mutex::new(CpuPercentCollector::new().unwrap()),
    });
    // Load agent config
    let config_path = std::path::PathBuf::from(CONFIG_FN);
    let config: Config = match config_path.is_file() {
        true => match read_config_toml(&config_path) {
            Ok(v) => v,
            Err(_) => {
                log::warn!("config file could not be read, falling back to defaults");
                Default::default()
            }
        },
        false => {
            log::warn!("config file could not be found, falling back to defaults");
            Default::default()
        }
    };
    // Create the HTTP server
    let bind = (config.host.clone(), config.port);
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(collector.clone())
            .app_data(config.clone())
            .service(get_all)
            .service(
                web::scope("/cpu").service(get_cpu).service(
                    web::scope("/load")
                        .service(get_cpu_load)
                        .service(get_cpu_load_average)
                        .service(get_cpu_load_per_core),
                ),
            )
            .service(
                web::scope("memory")
                    .service(get_memory)
                    .service(get_memory_perc_used)
                    .service(get_memory_detailed),
            )
    })
    .bind(bind)?
    .run()
    .await
}
