#[derive(Debug)]
pub struct EventProbe;

pub fn start_event_probe() -> Result<EventProbe, String> {
    Ok(EventProbe)
}
