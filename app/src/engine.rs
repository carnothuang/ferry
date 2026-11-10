// Conversion engine: runs ffmpeg as a subprocess, one job at a time,
// on a worker thread. The UI stays responsive and polls state each frame.

use crate::{App, JobKind, JobState};
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub enum EngineMsg {
    Started(u64),
    Finished(u64, bool, String), // id, success, note
    Idle,
}

pub struct Engine {
    pub tx: Sender<EngineMsg>,
    pub rx: Arc<Mutex<Receiver<EngineMsg>>>,
    pub busy: Arc<Mutex<bool>>,
}

impl Engine {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx: Arc::new(Mutex::new(rx)),
            busy: Arc::new(Mutex::new(false)),
        }
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

pub fn pump(ctx: &eframe::egui::Context, app: &mut App) {
    if app.engine.is_none() {
        app.engine = Some(Engine::new());
    }
    let (rx, tx, busy) = {
        let e = app.engine.as_ref().unwrap();
        (e.rx.clone(), e.tx.clone(), e.busy.clone())
    };

    // collect worker messages
    loop {
        let msg = rx.lock().unwrap().try_recv();
        match msg {
            Ok(EngineMsg::Started(id)) => {
                if let Some(job) = app.jobs.iter_mut().find(|j| j.id == id) {
                    job.state = JobState::Running;
                }
            }
            Ok(EngineMsg::Finished(id, ok, note)) => {
                if let Some(job) = app.jobs.iter_mut().find(|j| j.id == id) {
                    job.state = if ok { JobState::Done } else { JobState::Failed };
                    job.note = note;
                }
                ctx.request_repaint();
            }
            Ok(EngineMsg::Idle) => {}
            Err(_) => break,
        }
    }

    // dispatch the next pending job when idle
    if !*busy.lock().unwrap() {
        dispatch_next(&tx, &busy, app);
    }
}

fn dispatch_next(tx: &Sender<EngineMsg>, busy: &Arc<Mutex<bool>>, app: &mut App) {
    let next = app
        .jobs
        .iter()
        .find(|j| j.state == JobState::Pending)
        .map(|j| {
            (
                j.id,
                j.source.clone(),
                j.target.clone(),
                j.target_path.clone(),
                j.resize,
            )
        });
    let Some((id, source, target, target_path, resize)) = next else {
        return;
    };

    // resolve the same-name policy at dispatch time
    if target_path.exists() {
        match app.settings.collision {
            crate::Collision::Overwrite => {}
            crate::Collision::Skip => {
                if let Some(job) = app.jobs.iter_mut().find(|j| j.id == id) {
                    job.state = JobState::Skipped;
                    job.note = "同名文件已存在".into();
                }
                return;
            }
            crate::Collision::Rename => {
                if let Some(job) = app.jobs.iter_mut().find(|j| j.id == id) {
                    let dir = job.target_path.parent().unwrap().to_path_buf();
                    let stem = job
                        .target_path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "out".into());
                    let ext = job
                        .target_path
                        .extension()
                        .map(|e| e.to_string_lossy().to_string())
                        .unwrap_or_else(|| "bin".into());
                    let mut n = 1;
                    loop {
                        let cand = dir.join(format!("{stem}_{n}.{ext}"));
                        if !cand.exists() {
                            job.target_path = cand;
                            break;
                        }
                        n += 1;
                    }
                }
            }
        }
    }

    let target_path = app
        .jobs
        .iter()
        .find(|j| j.id == id)
        .map(|j| j.target_path.clone())
        .unwrap_or(target_path);
    let quality = crate::AUDIO_TIERS
        .get(app.settings.audio_quality)
        .map(|t| t.1)
        .unwrap_or(192);
    let kind = app
        .jobs
        .iter()
        .find(|j| j.id == id)
        .map(|j| j.kind)
        .unwrap_or(JobKind::Convert);

    spawn_job(
        tx,
        busy,
        JobSpec {
            id,
            kind,
            source,
            target,
            target_path,
            quality,
            resize,
        },
    );
}

struct JobSpec {
    id: u64,
    kind: JobKind,
    source: PathBuf,
    target: String,
    target_path: PathBuf,
    quality: u32,
    resize: Option<(u32, u32)>,
}

fn spawn_job(tx: &Sender<EngineMsg>, busy: &Arc<Mutex<bool>>, job: JobSpec) {
    *busy.lock().unwrap() = true;
    let tx = tx.clone();
    let busy = busy.clone();

    std::thread::Builder::new()
        .name("engine".into())
        .spawn(move || {
            let JobSpec {
                id,
                kind,
                source,
                target,
                target_path,
                quality,
                resize,
            } = job;
            let _ = tx.send(EngineMsg::Started(id));
            let res = match kind {
                JobKind::Convert => run_job(&source, &target, &target_path, quality),
                JobKind::Image => {
                    let r = crate::imaging::convert(&source, &target, &target_path, resize);
                    (r.ok, r.note)
                }
            };
            let _ = tx.send(EngineMsg::Finished(id, res.0, res.1));
            *busy.lock().unwrap() = false;
            let _ = tx.send(EngineMsg::Idle);
        })
        .ok();
}

pub fn run_job(
    source: &std::path::Path,
    target: &str,
    target_path: &std::path::Path,
    quality: u32,
) -> (bool, String) {
    // Build the ffmpeg command line from the mechanics sheet (see MECHANICS.md).
    let mut args: Vec<String> = vec![
        "-y".into(),
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-i".into(),
        source.to_string_lossy().into(),
    ];

    match target {
        "mp4" => {
            args.extend(video_args_h264());
            args.extend(audio_args_aac());
            args.push("-movflags".into());
            args.push("+faststart".into());
        }
        "mkv" | "mov" | "avi" => {
            args.extend(video_args_h264());
            args.extend(audio_args_aac());
        }
        "wmv" => {
            args.extend(["-c:v".into(), "wmv2".into(), "-b:v".into(), "2000k".into()]);
            args.extend(["-c:a".into(), "wmav2".into(), "-b:a".into(), "128k".into()]);
        }
        "webm" => {
            args.extend([
                "-c:v".into(),
                "libvpx-vp9".into(),
                "-crf".into(),
                "34".into(),
                "-b:v".into(),
                "0".into(),
            ]);
            args.extend(["-c:a".into(), "libopus".into(), "-b:a".into(), "96k".into()]);
        }
        "gif" => {
            // 15 fps, 480 px wide, 16-color palette, max 30 s (see MECHANICS.md)
            args.extend([
                "-t".into(),
                "30".into(),
                "-vf".into(),
                "fps=15,scale=480:-1:flags=lanczos,palettegen=max_colors=16".into(),
            ]);
            args.push("-loop".into());
            args.push("0".into());
        }
        "mp3" => {
            args.extend([
                "-vn".into(),
                "-c:a".into(),
                "libmp3lame".into(),
                "-b:a".into(),
                format!("{quality}k"),
            ]);
        }
        "aac" | "m4a" => {
            args.extend([
                "-vn".into(),
                "-c:a".into(),
                "aac".into(),
                "-b:a".into(),
                format!("{quality}k"),
            ]);
        }
        "wav" => {
            args.extend(["-vn".into(), "-c:a".into(), "pcm_s16le".into()]);
        }
        "flac" => {
            args.extend(["-vn".into(), "-c:a".into(), "flac".into()]);
        }
        "ogg" => {
            args.extend([
                "-vn".into(),
                "-c:a".into(),
                "libvorbis".into(),
                "-q:a".into(),
                "5".into(),
            ]);
        }
        _ => {
            return (false, format!("未知目标格式: {target}"));
        }
    }

    args.push(target_path.to_string_lossy().into());

    match Command::new("ffmpeg").args(&args).output() {
        Ok(o) if o.status.success() => (true, "完成".into()),
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr);
            let short = err.lines().last().unwrap_or("ffmpeg 返回失败").to_string();
            (false, short)
        }
        Err(e) => (false, format!("无法启动 ffmpeg: {e}")),
    }
}

fn video_args_h264() -> Vec<String> {
    vec![
        "-c:v".into(),
        "libx264".into(),
        "-crf".into(),
        "23".into(),
        "-preset".into(),
        "medium".into(),
    ]
}

fn audio_args_aac() -> Vec<String> {
    vec!["-c:a".into(), "aac".into(), "-b:a".into(), "128k".into()]
}

/// Join N files into one (concat demuxer). copy = stream copy, else re-encode.
pub fn run_join(files: &[PathBuf], out: &std::path::Path, copy: bool) -> Result<(), String> {
    let list = crate::tools::write_concat_list(files)?;
    let mut args: Vec<String> = vec![
        "-y".into(),
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-f".into(),
        "concat".into(),
        "-safe".into(),
        "0".into(),
        "-i".into(),
        list.to_string_lossy().into(),
    ];
    if copy {
        args.push("-c".into());
        args.push("copy".into());
    } else {
        args.extend(video_args_h264());
        args.extend(audio_args_aac());
    }
    args.push(out.to_string_lossy().into());
    run_checked(args)
}

/// Mux a video and an audio file into one container (stream copy).
pub fn run_mux(
    video: &std::path::Path,
    audio: &std::path::Path,
    out: &std::path::Path,
) -> Result<(), String> {
    let args = vec![
        "-y".to_string(),
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-i".into(),
        video.to_string_lossy().into(),
        "-i".into(),
        audio.to_string_lossy().into(),
        "-map".into(),
        "0:v:0".into(),
        "-map".into(),
        "1:a:0".into(),
        "-c".into(),
        "copy".into(),
        out.to_string_lossy().into(),
    ];
    run_checked(args)
}

fn run_checked(args: Vec<String>) -> Result<(), String> {
    match Command::new("ffmpeg").args(&args).output() {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr);
            Err(err.lines().last().unwrap_or("ffmpeg 失败").to_string())
        }
        Err(e) => Err(format!("无法启动 ffmpeg: {e}")),
    }
}
