use crate::app::CaptureSnapshotsHandler;
use log::{debug, error};
use std::time::Duration;
use tokio::time::sleep;

static TICK_EVERY: Duration = Duration::from_secs(60);

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
