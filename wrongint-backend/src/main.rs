use clap::{Command, arg};
use env_logger::Env;
use log::{error, info};
use std::sync::Arc;
use std::time::Duration;
use wrongint_backend::adapters::sources::{HackerNews, Lobsters, new_client};
use wrongint_backend::adapters::{ConfigLoader, Metrics, db};
use wrongint_backend::app::{self, IngestService, QueryService};
use wrongint_backend::config::Config;
use wrongint_backend::domain::SourceId;
use wrongint_backend::domain::time::DateTime;
use wrongint_backend::errors::Result;
use wrongint_backend::ports::http::{self, AppState};
use wrongint_backend::ports::timers::Scheduler;

fn cli() -> Command {
    Command::new("wrongint")
        .about("Measures how argumentative programming communities are.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("run")
                .about("Runs the sampler + HTTP API")
                .arg(arg!(<CONFIG> "Path to the configuration file"))
                .arg(arg!(--"sample-now" "Fire one ingest tick immediately on startup")),
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
        let report = service.ingest.ingest_tick(DateTime::now()).await;
        info!("startup --sample-now tick: {report}");
    }

    tokio::join!(
        service.scheduler.run(),
        http_server_loop(&service.http_server),
    );
    Ok(())
}

async fn http_server_loop(server: &http::Server<'_>) {
    loop {
        match server.run().await {
            Ok(_) => error!("http server exited without an error"),
            Err(err) => error!("http server exited with an error: {err}"),
        }
    }
}

struct Service<'a> {
    http_server: http::Server<'a>,
    scheduler: Scheduler,
    ingest: Arc<IngestService>,
}

impl<'a> Service<'a> {
    fn new(config: &'a Config) -> Result<Self> {
        let metrics = Metrics::new()?;
        let registry = metrics.registry().clone();

        let store: Arc<dyn app::Store> = Arc::new(db::Database::new(config.database_path())?);

        let client = new_client(config.user_agent(), config.request_timeout_secs())?;
        let sources: Vec<Arc<dyn app::Source>> = vec![
            Arc::new(HackerNews::new(client.clone(), config.hn_front_page_len())),
            Arc::new(Lobsters::new(client.clone())),
        ];

        let metrics: Arc<dyn app::Metrics> = Arc::new(metrics);
        let ingest = Arc::new(IngestService::new(
            sources,
            store.clone(),
            metrics,
            config.request_retries(),
        ));
        let query = Arc::new(QueryService::new(
            store,
            vec![SourceId::HackerNews, SourceId::Lobsters],
        ));

        let state = AppState::new(query, registry);
        let http_server = http::Server::new(config, state);
        let scheduler = Scheduler::new(
            ingest.clone(),
            Duration::from_secs(config.sample_interval_secs()),
        );

        Ok(Self {
            http_server,
            scheduler,
            ingest,
        })
    }
}
