#[derive(Debug)]
pub struct EventProbe;

pub fn start_event_probe<F>(_handler: F) -> Result<EventProbe, String>
where
    F: Fn(super::DesktopEvent) + Send + 'static,
{
    Ok(EventProbe)
}
