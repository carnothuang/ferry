// Tools: file pickers (zenity dialog out-of-process, plus editable text lists)
// and a one-shot ffmpeg availability probe.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;

static PICKER_RUNNING: AtomicBool = AtomicBool::new(false);
static FFMPEG_PROBE_STARTED: AtomicBool = AtomicBool::new(false);

static PICK_RESULT: Mutex<Option<Result<Vec<PathBuf>, String>>> = Mutex::new(None);
static FFMPEG_PROBE: Mutex<Option<Result<String, String>>> = Mutex::new(None);

pub fn picker_busy() -> bool {
    PICKER_RUNNING.load(Ordering::Relaxed)
}

pub fn take_pick_result() -> Option<Result<Vec<PathBuf>, String>> {
    PICK_RESULT.lock().ok()?.take()
}

/// Open a multi-file picker. Spawns zenity (or kdialog) as a child process so the
/// app itself never links against GTK. The result is collected on later frames.
pub fn open_file_picker(title: String, filter: Option<String>) {
    if PICKER_RUNNING.swap(true, Ordering::Relaxed) {
        return;
    }
    // A stale result from a dialog opened on another page must not be
    // mistaken for this dialog's answer.
    *PICK_RESULT.lock().unwrap() = None;
    let _ = std::thread::Builder::new()
        .name("picker".into())
        .spawn(move || {
            let out = run_zenity(title, filter);
            *PICK_RESULT.lock().unwrap() = Some(out);
            PICKER_RUNNING.store(false, Ordering::Relaxed);
        });
}

fn run_zenity(title: String, filter: Option<String>) -> Result<Vec<PathBuf>, String> {
    let mut cmd = Command::new("zenity");
    cmd.arg("--file-selection")
        .arg("--multiple")
        .arg("--title")
        .arg(title)
        .arg("--separator=\n")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(f) = filter {
        cmd.arg("--file-filter").arg(f);
    }
    // always allow choosing anything, in case the type filter misses
    cmd.arg("--file-filter").arg("所有文件 | *");
    let out = cmd
        .output()
        .map_err(|e| format!("无法启动文件选择器: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let files: Vec<PathBuf> = text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect();
    if out.status.success() && !files.is_empty() {
        Ok(files)
    } else {
        Err("已取消".into())
    }
}

/// Open a save-file picker (single target file), used by the mux tool.
#[allow(dead_code)]
pub fn open_save_picker(title: String, default_name: String, filter: Option<String>) {
    if PICKER_RUNNING.swap(true, Ordering::Relaxed) {
        return;
    }
    *PICK_RESULT.lock().unwrap() = None;
    let _ = std::thread::Builder::new()
        .name("picker".into())
        .spawn(move || {
            let out = run_zenity_save(title, default_name, filter);
            match out {
                Ok(Some(p)) => {
                    *PICK_RESULT.lock().unwrap() = Some(Ok(vec![p]));
                }
                Ok(None) => {
                    *PICK_RESULT.lock().unwrap() = Some(Err("已取消".into()));
                }
                Err(e) => {
                    *PICK_RESULT.lock().unwrap() = Some(Err(e));
                }
            }
            PICKER_RUNNING.store(false, Ordering::Relaxed);
        });
}

#[allow(dead_code)]
fn run_zenity_save(
    title: String,
    default_name: String,
    filter: Option<String>,
) -> Result<Option<PathBuf>, String> {
    let mut cmd = Command::new("zenity");
    cmd.arg("--file-selection")
        .arg("--save")
        .arg("--confirm-overwrite")
        .arg("--title")
        .arg(title)
        .arg("--filename")
        .arg(&default_name)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(f) = filter {
        cmd.arg("--file-filter").arg(f);
    }
    cmd.arg("--file-filter").arg("所有文件 | *");
    let out = cmd
        .output()
        .map_err(|e| format!("无法启动文件选择器: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if out.status.success() && !text.is_empty() {
        Ok(Some(PathBuf::from(text)))
    } else {
        Ok(None)
    }
}

/// Open a folder picker (settings: output directory).
pub fn open_dir_picker(title: String) {
    if PICKER_RUNNING.swap(true, Ordering::Relaxed) {
        return;
    }
    *PICK_RESULT.lock().unwrap() = None;
    let _ = std::thread::Builder::new()
        .name("picker".into())
        .spawn(move || {
            let mut cmd = Command::new("zenity");
            cmd.arg("--file-selection")
                .arg("--directory")
                .arg("--title")
                .arg(title)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            let out = cmd.output();
            let res = match out {
                Ok(o) if o.status.success() => {
                    let t = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    if t.is_empty() {
                        Err("已取消".into())
                    } else {
                        Ok(vec![PathBuf::from(t)])
                    }
                }
                _ => Err("已取消".into()),
            };
            *PICK_RESULT.lock().unwrap() = Some(res);
            PICKER_RUNNING.store(false, Ordering::Relaxed);
        });
}

pub fn ffmpeg_probe_started() -> bool {
    FFMPEG_PROBE_STARTED.load(Ordering::Relaxed)
}

pub fn take_ffmpeg_probe_result() -> Option<Result<String, String>> {
    FFMPEG_PROBE.lock().ok()?.take()
}

/// Ask ffmpeg for its version in the background; shown on the toolbar.
pub fn start_ffmpeg_probe() {
    if FFMPEG_PROBE_STARTED.swap(true, Ordering::Relaxed) {
        return;
    }
    let (tx, rx): (
        Sender<Result<String, String>>,
        Receiver<Result<String, String>>,
    ) = mpsc::channel();
    std::thread::spawn(move || {
        let res = match Command::new("ffmpeg").arg("-version").output() {
            Ok(o) if o.status.success() => {
                let text = String::from_utf8_lossy(&o.stdout);
                let first = text.lines().next().unwrap_or("ffmpeg").to_string();
                Ok(first)
            }
            _ => Err("未找到 ffmpeg".into()),
        };
        let _ = tx.send(res);
    });
    std::thread::spawn(move || {
        if let Ok(r) = rx.recv() {
            *FFMPEG_PROBE.lock().unwrap() = Some(r);
        }
    });
}

/// Escape a path for the concat demuxer list file (single quotes doubled).
pub fn concat_escape(p: &std::path::Path) -> String {
    format!("file '{}'", p.to_string_lossy().replace('\'', "'\\''"))
}

/// Write a concat list file; returns its path (in the system temp dir).
pub fn write_concat_list(files: &[PathBuf]) -> Result<PathBuf, String> {
    let mut list = String::new();
    for f in files {
        list.push_str(&concat_escape(f));
        list.push('\n');
    }
    let mut path = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    path.push(format!("ferry_concat_{stamp}.txt"));
    let mut f = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    f.write_all(list.as_bytes()).map_err(|e| e.to_string())?;
    Ok(path)
}
