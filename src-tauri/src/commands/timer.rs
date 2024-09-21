use tauri::{command, Runtime, State, Window, Manager, PhysicalPosition};

use crate::{
  state::{Pomodoro, TimerMode},
  PomodoroState, SettingsState, TimerState, TimerStatePayload,
};

fn update_pomodoro_state(state: &Pomodoro) -> Pomodoro {
  match state.mode {
    TimerMode::Relax => Pomodoro {
      mode: TimerMode::Work,
      cycles: state.cycles + 1,
    },
    TimerMode::Work => Pomodoro {
      mode: TimerMode::Relax,
      cycles: state.cycles,
    },
  }
}

#[command]
pub fn trigger_tray_menu<R: Runtime>(app: tauri::AppHandle<R>) {
    // 手动触发托盘菜单显示（这通常是响应点击事件）
    // 但是在 Tauri 中没有直接展示托盘菜单的方法，可以执行一些与托盘有关的逻辑
    let window = app.get_window(crate::MAIN_WINDOW_LABEL).unwrap();
    if let Some(monitor) = window.primary_monitor().unwrap() {
      // 获取屏幕尺寸
      let screen_size = monitor.size();

      // 获取窗口尺寸
      let window_size = window.outer_size().unwrap();

      // 计算窗口居中的坐标
      let x = (screen_size.width - window_size.width) / 2;
      let y = (screen_size.height - window_size.height) / 2;

      // 设置窗口位置为居中
      window.set_position(PhysicalPosition::new(x, y)).unwrap();
  }
    window.show().unwrap(); // 显示主窗口
}

#[command]
pub fn toggle_timer<R: Runtime>(
  window: Window<R>,
  timer: State<'_, TimerState>,
) -> Result<(), String> {
  timer
    .toggle()
    .map_err(|_| "Failed to toggle timer".to_string())?;

  window
    .emit("timer-running-change", *timer.is_running())
    .map_err(|_| "Failed to communicate running state".to_string())
}

#[command]
pub fn reset_timer<R: Runtime>(
  window: Window<R>,
  timer: State<'_, TimerState>,
  pomodoro_state: State<'_, PomodoroState>,
  settings: State<'_, SettingsState>,
) -> Result<(), String> {
  let pomodoro_state = pomodoro_state.lock().unwrap();

  let new_duration = pomodoro_state.duration(&settings.read().unwrap());
  timer
    .reset(new_duration)
    .map_err(|_| "Failed to reset timer".to_string())?;
  timer
    .resume()
    .map_err(|_| "Failed to resume timer".to_string())?;

  window
    .emit::<TimerStatePayload>(
      "timer-state",
      TimerStatePayload {
        mode: pomodoro_state.mode,
        cycle: pomodoro_state.cycles,
        is_ended: false,
        duration_secs: new_duration.as_secs() as u32,
      },
    )
    .map_err(|_| "Failed to communicate new state".to_string())?;

  window
    .emit("timer-running-change", *timer.is_running())
    .map_err(|_| "Failed to communicate running state".to_string())
}

#[command]
pub fn next_timer_cycle<R: Runtime>(
  window: Window<R>,
  timer: State<'_, TimerState>,
  pomodoro_state: State<'_, PomodoroState>,
  settings: State<'_, SettingsState>,
) -> Result<(), String> {
  let mut pomodoro_state = pomodoro_state.lock().unwrap();

  *pomodoro_state = update_pomodoro_state(&pomodoro_state);
  let new_duration = pomodoro_state.duration(&settings.read().unwrap());
  timer
    .reset(new_duration)
    .map_err(|_| "Failed to reset timer".to_string())?;
  timer
    .resume()
    .map_err(|_| "Failed to resume timer".to_string())?;

  window
    .emit::<TimerStatePayload>(
      "timer-state",
      TimerStatePayload {
        mode: pomodoro_state.mode,
        cycle: pomodoro_state.cycles,
        is_ended: false,
        duration_secs: new_duration.as_secs() as u32,
      },
    )
    .map_err(|_| "Failed to communicate new state".to_string())?;

  window
    .emit("timer-running-change", *timer.is_running())
    .map_err(|_| "Failed to communicate running state".to_string())
}

#[command]
pub fn get_timer_state(
  settings: State<'_, SettingsState>,
  pomodoro_state: State<'_, PomodoroState>,
) -> TimerStatePayload {
  let settings = settings.read().unwrap();
  let pomodoro_state = pomodoro_state.lock().unwrap();

  TimerStatePayload {
    mode: pomodoro_state.mode,
    cycle: pomodoro_state.cycles,
    is_ended: false,
    duration_secs: pomodoro_state.duration(&settings).as_secs() as u32,
  }
}
