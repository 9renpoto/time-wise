use std::sync::mpsc;

use super::DesktopEvent;

#[derive(Debug)]
pub struct EventProbe;

pub fn start_event_probe() -> Result<(EventProbe, mpsc::Receiver<DesktopEvent>), String> {
    let (sender, receiver) = mpsc::channel();
    drop(sender);
    Ok((EventProbe, receiver))
}
