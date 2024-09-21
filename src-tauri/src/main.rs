#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

use commands::settings::*;
use commands::timer::*;
use helpers::fs::load_settings;
use helpers::shortcuts::setup_shortcuts;
use helpers::sound::SoundPlayer;
use helpers::timer::create_timer_listener;
use serde::Serialize;
use state::{Pomodoro, Settings};
use tauri::{Manager, RunEvent};
use tauri_plugin_autostart::MacosLauncher;
use ticking_timer::Timer;
use ts_rs::TS;
use ui::tray::setup_tray;
use ui::window::setup_main_window;

mod commands;
mod helpers;
mod state;
mod ui;

use crate::state::TimerMode;

#[derive(Clone, Serialize, TS)]
#[ts(export)]
pub struct TimerStatePayload {
  mode: TimerMode,
  cycle: u32,
  is_ended: bool,
  duration_secs: u32,
}

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const SETTINGS_WINDOW_LABEL: &str = "settings";
pub const ABOUT_WINDOW_LABEL: &str = "about";

pub type TimerState = Arc<Timer>;
pub type SettingsState = RwLock<Settings>;
pub type PomodoroState = Mutex<Pomodoro>;

fn create_audio_notification_thread<R: tauri::Runtime>(
  app_handle: &tauri::AppHandle<R>,
  timer_end_receiver: mpsc::Receiver<()>,
) {
  let bell_audio_path = app_handle
    .path_resolver()
    .resolve_resource("resources/audio/bell.mp3")
    .map(PathBuf::into_os_string)
    .and_then(|s| s.into_string().ok());

  thread::spawn({
    let app_handle = app_handle.clone();
    move || match SoundPlayer::new() {
      Ok(sound_player) => match bell_audio_path {
        Some(bell_audio_path) => {
          for _ in timer_end_receiver {
            let settings_state = app_handle.state::<SettingsState>();
            let settings_state = settings_state.read().unwrap();
            if settings_state.should_play_sound == Some(true) {
              sound_player.play(bell_audio_path.clone()).unwrap();
            }
          }
        }
        None => eprintln!("Can't resolve bell sound file path"),
      },
      Err(error) => eprintln!("Unable to initialize sound player due to error {:?}", error),
    }
  });
}

fn create_app<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::App<R> {
  builder
    .menu(tauri::Menu::new())
    .manage::<SettingsState>(RwLock::new(Settings::default()))
    .manage::<PomodoroState>(Mutex::new(Pomodoro {
      cycles: 0,
      mode: state::TimerMode::Work,
    }))
    .setup(|app| {
      let app_handle = app.handle();
      let main_window = setup_main_window(&app_handle).unwrap();

      #[cfg(not(test))]
      crate::ui::window::decorate_window(&main_window);

      #[cfg(debug_assertions)]
      main_window.open_devtools();

      let (timer_end_sender, timer_end_receiver) = mpsc::sync_channel(1);

      let timer = Timer::new(
        Duration::from_millis(100),
        create_timer_listener(&app_handle, timer_end_sender),
      );

      create_audio_notification_thread(&app_handle, timer_end_receiver);

      app_handle.manage::<TimerState>(Arc::new(timer));

      {
        let mut work_duration = app.state::<SettingsState>().read().unwrap().work_duration;
        match load_settings(&app_handle) {
          Ok(settings) => {
            work_duration = settings.work_duration;
            *app.state::<SettingsState>().write().unwrap() = settings;
          }
          Err(error) => {
            eprintln!("Failed to load settings with error {:?}", error);
          }
        }
        app.state::<TimerState>().reset(work_duration).unwrap();
      }

      setup_tray(app);

      Ok(())
    })
    .plugin(tauri_plugin_autostart::init(
      MacosLauncher::LaunchAgent,
      None,
    ))
    .invoke_handler(tauri::generate_handler![
      toggle_timer,
      reset_timer,
      next_timer_cycle,
      get_timer_state,
      get_settings,
      set_settings,
      trigger_tray_menu
    ])
    .build(tauri::generate_context!())
    .expect("Error while building tauri application")
}

fn main() {
  let app = create_app(tauri::Builder::default());
  app.run(move |app_handle, e| {
    if matches!(e, RunEvent::Ready) {
      setup_shortcuts(app_handle);
    }
  });
}

#[cfg(test)]
mod tests {
  use super::*;
  use tauri::Manager;

  #[test]
  fn it_creates_main_window() {
    let app = create_app(tauri::test::mock_builder());
    assert!(app.get_window(MAIN_WINDOW_LABEL).is_some());
  }
}
