//! Alarm: a reminder marked as alarm rings (a macOS system sound, repeated) and keeps Duck on screen
//! until the user stops or snoozes it. It only rings while Duck is open and the Mac is awake.
use serde::Serialize;
use std::{
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};

use crate::{desktop::DesktopState, storage::{self, Db, Result}};

const SOUND: &str = "/System/Library/Sounds/Hero.aiff";
/// A forgotten alarm does not ring forever.
const MAX_RING: Duration = Duration::from_secs(180);
const PAUSE: Duration = Duration::from_millis(900);
const POLL: Duration = Duration::from_millis(80);
/// The system sounds last about two seconds. If one has not finished by now the audio device is stuck
/// (for example right after the Mac wakes up), so it is cut and tried again.
const MAX_PLAY: Duration = Duration::from_secs(10);
const DEFAULT_SNOOZE_MINUTES: i64 = 5;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ringing {
    pub id: String,
    pub title: String,
}

static RINGING: Mutex<Vec<Ringing>> = Mutex::new(Vec::new());
static RINGER_RUNNING: AtomicBool = AtomicBool::new(false);

fn lock() -> std::sync::MutexGuard<'static, Vec<Ringing>> {
    RINGING.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Adds an alarm unless it is already ringing. Returns whether the list changed.
fn add(list: &mut Vec<Ringing>, id: &str, title: &str) -> bool {
    if list.iter().any(|alarm| alarm.id == id) {
        return false;
    }
    list.push(Ringing { id: id.into(), title: title.into() });
    true
}

fn remove(list: &mut Vec<Ringing>, id: &str) -> bool {
    let before = list.len();
    list.retain(|alarm| alarm.id != id);
    list.len() != before
}

pub fn is_ringing() -> bool {
    !lock().is_empty()
}

/// Plays the sound once. Returns false if the sound could not start. Stops early when `keep_going` says so.
fn play_once(volume: &str, keep_going: &dyn Fn() -> bool) -> bool {
    let Ok(mut child) = Command::new("/usr/bin/afplay")
        .args(["-v", volume, SOUND])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) => {}
            Err(_) => return false,
        }
        if started.elapsed() > MAX_PLAY {
            let _ = child.kill();
            let _ = child.wait();
            return false;
        }
        if !keep_going() {
            let _ = child.kill();
            let _ = child.wait();
            return true;
        }
        std::thread::sleep(POLL);
    }
}

fn wait(total: Duration, keep_going: &dyn Fn() -> bool) {
    let start = Instant::now();
    while start.elapsed() < total && keep_going() {
        std::thread::sleep(POLL);
    }
}

fn announce(app: &AppHandle) {
    let _ = app.emit("alarm-changed", lock().clone());
}

/// Lets the mascot leave (or stay, if it is always visible) once nothing rings any more.
fn release_mascot(app: &AppHandle) {
    if let Ok(mut desktop) = app.state::<DesktopState>().lock() {
        desktop.temporary_until = Some(Instant::now() + Duration::from_millis(300));
    }
    let _ = crate::desktop::refresh_mascot(app, false);
}

fn ring_loop(app: AppHandle) {
    loop {
        let started = Instant::now();
        let alive = || is_ringing() && started.elapsed() < MAX_RING;
        let mut warned = false;
        while alive() {
            if !play_once("1.0", &alive) && !warned {
                // Keep trying: the audio device may only be busy. The visible alarm stays on screen meanwhile.
                warned = true;
                let _ = app.emit("notification-error", "No he podido reproducir el sonido de la alarma. Sigo intentándolo; la alarma sigue en pantalla.");
            }
            wait(PAUSE, &alive);
        }
        if is_ringing() {
            // Nobody answered: stop ringing but leave the reminder where it is.
            lock().clear();
            announce(&app);
            release_mascot(&app);
        }
        RINGER_RUNNING.store(false, Ordering::SeqCst);
        // An alarm may have arrived while the loop was finishing.
        if !(is_ringing() && !RINGER_RUNNING.swap(true, Ordering::SeqCst)) {
            break;
        }
    }
}

/// Starts ringing for a reminder that has just become due.
pub fn start_ringing(app: &AppHandle, reminder_id: &str, title: &str) {
    let changed = add(&mut lock(), reminder_id, title);
    if changed {
        announce(app);
    }
    // Keep Duck visible for as long as the alarm can ring.
    if let Ok(mut desktop) = app.state::<DesktopState>().lock() {
        desktop.temporary_until = Some(Instant::now() + MAX_RING + Duration::from_secs(5));
        desktop.leaving = None;
    }
    let _ = crate::desktop::refresh_mascot(app, false);
    if !RINGER_RUNNING.swap(true, Ordering::SeqCst) {
        let app = app.clone();
        std::thread::spawn(move || ring_loop(app));
    }
}

/// Stops one alarm. Safe to call when it is not ringing.
pub fn stop(app: &AppHandle, reminder_id: &str) {
    let (changed, empty) = {
        let mut list = lock();
        (remove(&mut list, reminder_id), list.is_empty())
    };
    if changed {
        announce(app);
        if empty {
            release_mascot(app);
        }
    }
}

#[tauri::command]
pub fn ringing_alarms() -> Vec<Ringing> {
    lock().clone()
}

#[tauri::command]
pub fn stop_alarm(app: AppHandle, id: String) -> Result<()> {
    stop(&app, &id);
    Ok(())
}

/// Silences the alarm and schedules the same reminder again. The sound stops even if the reminder is gone.
#[tauri::command]
pub fn snooze_alarm(app: AppHandle, id: String, minutes: Option<i64>, db: tauri::State<Db>) -> Result<()> {
    stop(&app, &id);
    let result = storage::snooze_reminder(&*db.lock().map_err(storage::err)?, &id, minutes.unwrap_or(DEFAULT_SNOOZE_MINUTES));
    let _ = app.emit("data-changed", ());
    result.map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn the_same_alarm_is_not_listed_twice_and_can_be_removed() {
        let mut list = Vec::new();
        assert!(add(&mut list, "a", "Despertar"));
        assert!(!add(&mut list, "a", "Despertar"));
        assert!(add(&mut list, "b", "Pastilla"));
        assert_eq!(list.len(), 2);
        assert!(remove(&mut list, "a"));
        assert!(!remove(&mut list, "a"));
        assert_eq!(list, vec![Ringing { id: "b".into(), title: "Pastilla".into() }]);
    }

    #[test]
    fn the_system_sound_exists_and_playing_never_blocks_longer_than_the_watchdog() {
        assert!(std::path::Path::new(SOUND).exists(), "falta {SOUND}");
        assert!(std::path::Path::new("/usr/bin/afplay").exists());
        let start = Instant::now();
        // Whether the audio device answers or is stuck (after wake-up, say), this must come back in bounded time.
        let _ = play_once("0", &|| true);
        assert!(start.elapsed() < MAX_PLAY + Duration::from_secs(3), "tardó {:?}", start.elapsed());
    }

    #[test]
    fn stopping_kills_the_sound_quickly() {
        let checks = AtomicUsize::new(0);
        let start = Instant::now();
        // Allow a few polls, then ask to stop while the sound is still playing.
        let keep_going = || checks.fetch_add(1, Ordering::SeqCst) < 3;
        assert!(play_once("0", &keep_going));
        assert!(start.elapsed() < Duration::from_secs(2), "tardó {:?}", start.elapsed());
    }
}
