//! Explicitly configured local services. Discovery never starts a process.
use crate::{now, path_id, Project, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, VecDeque},
    fs,
    io::{BufReader, Read},
    net::{SocketAddr, TcpStream},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub executable: String,
    pub args: Vec<String>,
    pub port: Option<u16>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub service_ids: Vec<String>,
}
#[derive(Default, Serialize, Deserialize)]
struct Config {
    services: Vec<Service>,
    workspaces: Vec<Workspace>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Line {
    seq: u64,
    time: u64,
    stream: String,
    text: String,
}
#[derive(Default)]
struct Log {
    lines: VecDeque<Line>,
    seq: u64,
}
struct Running {
    child: Child,
    log: Arc<Mutex<Log>>,
}
pub struct Center {
    path: PathBuf,
    config: Config,
    running: HashMap<String, Running>,
    finished: HashMap<String, (Option<i32>, Arc<Mutex<Log>>)>,
}

fn hidden(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
}
fn log_reader(reader: impl Read + Send + 'static, stream: &'static str, log: Arc<Mutex<Log>>) {
    std::thread::spawn(move || {
        let reader = BufReader::new(reader);
        let mut buffer = Vec::new();
        for byte in reader.bytes() {
            match byte {
                Ok(b) => {
                    if b == b'\n' || buffer.len() >= 4096 {
                        push_log(&log, stream, &buffer);
                        buffer.clear();
                    } else if b != b'\r' {
                        buffer.push(b);
                    }
                }
                Err(_) => break,
            }
        }
        if !buffer.is_empty() {
            push_log(&log, stream, &buffer);
        }
    });
}
fn push_log(log: &Arc<Mutex<Log>>, stream: &str, bytes: &[u8]) {
    if let Ok(mut log) = log.lock() {
        let text = String::from_utf8_lossy(bytes)
            .chars()
            .filter(|c| !c.is_control() || *c == '\t')
            .collect::<String>();
        log.seq += 1;
        let seq = log.seq;
        log.lines.push_back(Line {
            seq,
            time: now(),
            stream: stream.into(),
            text,
        });
        while log.lines.len() > 600 {
            log.lines.pop_front();
        }
    }
}
pub fn port_busy(port: u16) -> bool {
    TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(120),
    )
    .is_ok()
}
impl Center {
    pub fn open(path: PathBuf) -> Result<Self> {
        let config = if path.exists() {
            serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|_| "Servis yapılandırması okunamadı.")?
        } else {
            Config::default()
        };
        Ok(Self {
            path,
            config,
            running: HashMap::new(),
            finished: HashMap::new(),
        })
    }
    fn save(&self) -> Result<()> {
        let temp = self.path.with_extension("json.tmp");
        fs::write(
            &temp,
            serde_json::to_vec_pretty(&self.config).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        fs::rename(temp, &self.path).map_err(|e| e.to_string())
    }
    fn reap(&mut self) {
        let ended: Vec<_> = self
            .running
            .iter_mut()
            .filter_map(|(id, r)| match r.child.try_wait() {
                Ok(Some(status)) => Some((id.clone(), status.code())),
                _ => None,
            })
            .collect();
        for (id, code) in ended {
            if let Some(r) = self.running.remove(&id) {
                self.finished.insert(id, (code, r.log));
            }
        }
    }
    pub fn snapshot(&mut self) -> Value {
        self.reap();
        let services:Vec<_>=self.config.services.iter().map(|s|{
 let running=self.running.get(&s.id);let ended=self.finished.get(&s.id);json!({"id":s.id,"projectId":s.project_id,"name":s.name,"executable":s.executable,"args":s.args,"port":s.port,"status":if running.is_some(){"running"}else if ended.is_some(){"exited"}else{"stopped"},"pid":running.map(|r|r.child.id()),"exitCode":ended.and_then(|e|e.0)})}).collect();
        json!({"services":services,"workspaces":self.config.workspaces})
    }
    pub fn add(&mut self, args: &Value, projects: &[Project]) -> Result<Value> {
        let project_id = args["projectId"].as_str().ok_or("Proje gerekli.")?;
        if !projects.iter().any(|p| p.id == project_id) {
            return Err("Kayıtlı bir proje seç.".into());
        }
        if self.config.services.len() >= 100 {
            return Err("En fazla 100 servis tanımlanabilir.".into());
        }
        let name = args["name"].as_str().unwrap_or("").trim();
        let executable = args["executable"].as_str().unwrap_or("").trim();
        if name.is_empty() || name.len() > 100 || executable.is_empty() || executable.len() > 1000 {
            return Err("Servis adı ve çalıştırılacak program gerekli.".into());
        }
        let argv: Vec<String> = serde_json::from_value(args["args"].clone())
            .map_err(|_| "Argümanlar metin dizisi olmalı.")?;
        if argv.len() > 50
            || argv.iter().any(|a| a.len() > 4000 || a.contains('\0'))
            || executable.contains('\0')
        {
            return Err("Geçersiz komut uzunluğu.".into());
        }
        let port = match args["port"].as_u64() {
            Some(p) if p > 0 && p <= 65535 => Some(p as u16),
            Some(_) => return Err("Port 1–65535 arasında olmalı.".into()),
            None if args["port"].is_null() => None,
            None => return Err("Port bir tam sayı olmalı.".into()),
        };
        let id = path_id(&format!(
            "{project_id}:{name}:{:?}",
            std::time::SystemTime::now()
        ));
        self.config.services.push(Service {
            id: id.clone(),
            project_id: project_id.into(),
            name: name.into(),
            executable: executable.into(),
            args: argv,
            port,
        });
        self.save()?;
        Ok(json!({"id":id}))
    }
    pub fn start(&mut self, id: &str, projects: &[Project]) -> Result<Value> {
        self.reap();
        if self.running.contains_key(id) {
            return Err("Bu servis zaten çalışıyor.".into());
        }
        if self.running.len() >= 12 {
            return Err("Aynı anda en fazla 12 servis çalışabilir.".into());
        }
        let service = self
            .config
            .services
            .iter()
            .find(|s| s.id == id)
            .ok_or("Servis bulunamadı.")?
            .clone();
        let project = projects
            .iter()
            .find(|p| p.id == service.project_id)
            .ok_or("Servisin projesi artık kütüphanede yok.")?;
        if let Some(port) = service.port {
            if port_busy(port) {
                return Err(format!(
                    "{port} portu kullanımda. Mevcut süreç durdurulmadı."
                ));
            }
        }
        let mut command = Command::new(&service.executable);
        #[cfg(windows)]
        if service.executable == "npm" || service.executable == "npm.cmd" {
            if service.args.len() != 2
                || service.args[0] != "run"
                || service.args[1].is_empty()
                || !service.args[1]
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_:".contains(&b))
            {
                return Err("npm için yalnızca npm run <script> kullanılabilir.".into());
            }
            command = Command::new("cmd.exe");
            command
                .args(["/D", "/S", "/C"])
                .arg(format!("npm.cmd run {}", service.args[1]));
        } else {
            command.args(&service.args);
        }
        #[cfg(not(windows))]
        command.args(&service.args);
        command
            .current_dir(&project.path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        hidden(&mut command);
        let mut child = command
            .spawn()
            .map_err(|e| format!("Program başlatılamadı: {e}"))?;
        let pid = child.id();
        let log = Arc::new(Mutex::new(Log::default()));
        if let Some(out) = child.stdout.take() {
            log_reader(out, "stdout", log.clone());
        }
        if let Some(err) = child.stderr.take() {
            log_reader(err, "stderr", log.clone());
        }
        self.finished.remove(id);
        self.running.insert(id.into(), Running { child, log });
        Ok(json!({"pid":pid}))
    }
    pub fn stop(&mut self, id: &str) -> Result<Value> {
        self.reap();
        if let Some(r) = self.running.get_mut(id) {
            #[cfg(windows)]
            {
                let mut cmd = Command::new("taskkill.exe");
                cmd.args(["/PID", &r.child.id().to_string(), "/T", "/F"]);
                hidden(&mut cmd);
                let output = cmd.output().map_err(|e| e.to_string())?;
                if !output.status.success()
                    && r.child.try_wait().map_err(|e| e.to_string())?.is_none()
                {
                    return Err("Servis süreç ağacı durdurulamadı.".into());
                }
            }
            #[cfg(not(windows))]
            r.child.kill().map_err(|e| e.to_string())?;
            let _ = r.child.wait();
        }
        self.reap();
        Ok(json!(true))
    }
    pub fn logs(&mut self, id: &str) -> Result<Value> {
        self.reap();
        let log = self
            .running
            .get(id)
            .map(|r| &r.log)
            .or_else(|| self.finished.get(id).map(|r| &r.1));
        match log {
            Some(l) => Ok(json!(l.lock().map_err(|_| "Log kilidi alınamadı.")?.lines)),
            None => Ok(json!([])),
        }
    }
    pub fn delete(&mut self, id: &str) -> Result<Value> {
        self.reap();
        if self.running.contains_key(id) {
            return Err("Önce servisi durdur.".into());
        }
        self.config.services.retain(|s| s.id != id);
        for w in &mut self.config.workspaces {
            w.service_ids.retain(|s| s != id);
        }
        self.finished.remove(id);
        self.save()?;
        Ok(json!(true))
    }
    pub fn workspace_add(&mut self, args: &Value) -> Result<Value> {
        let name = args["name"].as_str().unwrap_or("").trim();
        let ids: Vec<String> = serde_json::from_value(args["serviceIds"].clone())
            .map_err(|_| "Servis seçimi gerekli.")?;
        if name.is_empty()
            || name.len() > 100
            || ids.is_empty()
            || ids.len() > 12
            || self.config.workspaces.len() >= 30
        {
            return Err("Çalışma alanı adı ve 1–12 servis gerekli (en fazla 30 alan).".into());
        }
        if ids
            .iter()
            .any(|id| !self.config.services.iter().any(|s| &s.id == id))
        {
            return Err("Bilinmeyen servis.".into());
        }
        let mut unique = ids;
        let mut seen = std::collections::HashSet::new();
        unique.retain(|id| seen.insert(id.clone()));
        let id = path_id(&format!("{name}:{:?}", std::time::SystemTime::now()));
        self.config.workspaces.push(Workspace {
            id: id.clone(),
            name: name.into(),
            service_ids: unique,
        });
        self.save()?;
        Ok(json!({"id":id}))
    }
    pub fn workspace_start(&mut self, id: &str, projects: &[Project]) -> Result<Value> {
        self.reap();
        let ids = self
            .config
            .workspaces
            .iter()
            .find(|w| w.id == id)
            .ok_or("Alan bulunamadı.")?
            .service_ids
            .clone();
        let mut started: Vec<String> = Vec::new();
        for service in ids {
            if self.running.contains_key(&service) {
                continue;
            }
            if let Err(error) = self.start(&service, projects) {
                for id in &started {
                    let _ = self.stop(id);
                }
                return Err(format!("Başlatma geri alındı: {error}"));
            }
            started.push(service);
        }
        Ok(json!({"started":started}))
    }
    pub fn workspace_stop(&mut self, id: &str) -> Result<Value> {
        let ids = self
            .config
            .workspaces
            .iter()
            .find(|w| w.id == id)
            .ok_or("Alan bulunamadı.")?
            .service_ids
            .clone();
        for id in ids {
            self.stop(&id)?;
        }
        Ok(json!(true))
    }
    pub fn workspace_delete(&mut self, id: &str) -> Result<Value> {
        self.config.workspaces.retain(|w| w.id != id);
        self.save()?;
        Ok(json!(true))
    }
    pub fn shutdown(&mut self) {
        let ids: Vec<_> = self.running.keys().cloned().collect();
        for id in ids {
            let _ = self.stop(&id);
        }
    }
}
impl Drop for Center {
    fn drop(&mut self) {
        self.shutdown();
    }
}
