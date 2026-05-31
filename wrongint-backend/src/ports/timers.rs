use crate::app::{CaptureSnapshotsHandler, UpdateMetricsHandler};
use log::{debug, error};
use std::time::Duration;
use tokio::time::sleep;

static TICK_EVERY: Duration = Duration::from_secs(60);
static METRICS_TICK_EVERY: Duration = Duration::from_secs(15);

pub struct CaptureSnapshotsTimer<H: CaptureSnapshotsHandler> {
    handler: H,
}

impl<H> CaptureSnapshotsTimer<H>
where
    H: CaptureSnapshotsHandler,
{
    pub fn new(handler: H) -> Self {
        Self { handler }
    }

    pub async fn run(&self) {
        loop {
            match self.handler.handle().await {
                Ok(_) => {
                    debug!("executed capture snapshots timer");
                }
                Err(err) => {
                    error!("error executing capture snapshots timer: {}", err);
                }
            }
            sleep(TICK_EVERY).await;
        }
    }
}

pub struct UpdateMetricsTimer<H: UpdateMetricsHandler> {
    handler: H,
}

impl<H> UpdateMetricsTimer<H>
where
    H: UpdateMetricsHandler,
{
    pub fn new(handler: H) -> Self {
        Self { handler }
    }

    pub async fn run(&self) {
        loop {
            match self.handler.handle().await {
                Ok(_) => {
                    debug!("executed update metrics timer");
                }
                Err(err) => {
                    error!("error executing update metrics timer: {}", err);
                }
            }
            sleep(METRICS_TICK_EVERY).await;
        }
    }
}
