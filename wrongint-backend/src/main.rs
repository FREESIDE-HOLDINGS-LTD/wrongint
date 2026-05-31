use clap::{Command, arg};
use env_logger::Env;
use log::{error, info};
use wrongint_backend::adapters::sources::{HackerNews, Lobsters, Sources, new_client};
use wrongint_backend::adapters::{ConfigLoader, Metrics, redb};
use wrongint_backend::app::CaptureSnapshotsHandler as _;
use wrongint_backend::app::capture_snapshots::CaptureSnapshotsHandler;
use wrongint_backend::app::get_index_series::GetIndexSeriesHandler;
use wrongint_backend::app::update_metrics::UpdateMetricsHandler;
use wrongint_backend::config::Config;
use wrongint_backend::errors::Result;
use wrongint_backend::ports::http::{self, AppState};
use wrongint_backend::ports::timers::{CaptureSnapshotsTimer, UpdateMetricsTimer};

fn cli() -> Command {
    Command::new("wrongint")
        .about("Measures how argumentative programming communities are.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("run")
                .about("Runs the sampler + HTTP API")
                .arg(arg!(<CONFIG> "Path to the configuration file"))
                .arg(arg!(--"sample-now" "Capture one round of snapshots immediately on startup")),
        )
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().filter_or("RUST_LOG", "info")).init();

    let matches = cli().get_matches();
    match matches.subcommand() {
        Some(("run", sub)) => {
            let config_path = sub.try_get_one::<String>("CONFIG")?.unwrap();
            let sample_now = sub.get_flag("sample-now");
            run(config_path, sample_now).await?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

async fn run(config_path: &str, sample_now: bool) -> Result<()> {
    let config = ConfigLoader::new(config_path).load()?;
    let service = Service::new(&config)?;

    if sample_now {
        info!("startup --sample-now: capturing snapshots");
        if let Err(err) = service.capture_handler.handle().await {
            error!("startup capture failed: {err}");
        }
    }

    tokio::join!(
        service.capture_timer.run(),
        service.update_metrics_timer.run(),
        http_server_loop(&service.http_server),
    );
    Ok(())
}

async fn http_server_loop<H>(server: &http::Server<'_, H>)
where
    H: wrongint_backend::app::GetIndexSeriesHandler + Clone + Send + Sync + 'static,
{
    loop {
        match server.run().await {
            Ok(_) => error!("http server exited without an error"),
            Err(err) => error!("http server exited with an error: {err}"),
        }
    }
}

type CaptureHandlerImpl = CaptureSnapshotsHandler<redb::Database, redb::Database, Sources, Metrics>;
type GetIndexSeriesHandlerImpl = GetIndexSeriesHandler<redb::Database, Metrics>;
type UpdateMetricsHandlerImpl = UpdateMetricsHandler<redb::Database, Metrics>;
type CaptureTimerImpl = CaptureSnapshotsTimer<CaptureHandlerImpl>;
type UpdateMetricsTimerImpl = UpdateMetricsTimer<UpdateMetricsHandlerImpl>;
type HttpServerImpl<'a> = http::Server<'a, GetIndexSeriesHandlerImpl>;

struct Service<'a> {
    http_server: HttpServerImpl<'a>,
    capture_timer: CaptureTimerImpl,
    update_metrics_timer: UpdateMetricsTimerImpl,
    capture_handler: CaptureHandlerImpl,
}

impl<'a> Service<'a> {
    fn new(config: &'a Config) -> Result<Self> {
        let metrics = Metrics::new()?;
        let registry = metrics.registry().clone();

        let database = redb::Database::new(config.database_path())?;

        let client = new_client(config.user_agent(), config.request_timeout_secs())?;
        let sources = Sources::new(
            HackerNews::new(client.clone(), config.hn_front_page_len()),
            Lobsters::new(client.clone()),
        );

        let capture_handler = CaptureSnapshotsHandler::new(
            database.clone(),
            database.clone(),
            sources,
            metrics.clone(),
        );
        let get_index_series_handler =
            GetIndexSeriesHandler::new(database.clone(), metrics.clone());
        let update_metrics_handler = UpdateMetricsHandler::new(database.clone(), metrics.clone());

        let capture_timer = CaptureSnapshotsTimer::new(capture_handler.clone());
        let update_metrics_timer = UpdateMetricsTimer::new(update_metrics_handler);
        let state = AppState::new(get_index_series_handler, registry);
        let http_server = http::Server::new(config, state);

        Ok(Self {
            http_server,
            capture_timer,
            update_metrics_timer,
            capture_handler,
        })
    }
}
