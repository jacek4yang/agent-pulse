//! Explicit integration probe, only run inside an isolated Xvfb desktop in CI.
#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use agent_pulse_lib::{executor::run_task_once, model::*, platform};
    if std::env::var("AGENT_PULSE_ISOLATED_DESKTOP_TEST").as_deref() != Ok("1") {
        return Err("This probe requires an isolated CI desktop".into());
    }
    let platform = platform::automation();
    platform.check_available()?;
    let mut task = ScheduledTask::new(
        "isolated Enter probe",
        WindowTarget {
            title: Some("AgentPulseIsolatedProbe".into()),
            title_match_mode: TitleMatchMode::Exact,
            ..Default::default()
        },
        Schedule::After {
            hours: 0,
            minutes: 0,
            seconds: 1,
        },
        vec![
            Action::TypeText {
                text: "pulse probe 123".into(),
            },
            Action::PressKey {
                key: Key::Enter,
                count: 1,
                interval_ms: 0,
            },
        ],
        MisfirePolicy::RunImmediately,
        chrono::Utc::now(),
    )?;
    let record = run_task_once(
        platform.as_ref(),
        &mut task,
        &Settings::default(),
        chrono::Utc::now(),
        &|_| {},
    )?;
    if let ExecutionOutcome::Failure { error_message, .. } = record.outcome {
        return Err(error_message.into());
    }
    println!("Keyboard sequence accepted by the isolated target");
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("This probe runs only on an isolated Linux X11 desktop.");
}
