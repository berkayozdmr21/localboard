//! Shared native core. It never uploads paths, source code or logs.
pub mod processes;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub id: String,
    pub path: String,
    pub scanned_at: u64,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub root_id: String,
    pub name: String,
    pub path: String,
    pub stack: Vec<String>,
    pub scripts: Vec<String>,
    pub favorite: bool,
}
#[derive(Clone, Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub roots: Vec<Root>,
    pub projects: Vec<Project>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GitChange {
    pub status: String,
    pub path: String,
}
#[derive(Clone, Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GitInfo {
    pub available: bool,
    pub branch: String,
    pub changes: Vec<GitChange>,
    pub commits: Vec<Value>,
    pub error: Option<String>,
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub fn path_id(path: &str) -> String {
    format!("{:x}", Sha256::digest(path.as_bytes()))[..20].to_string()
}
fn path_text(path: &Path) -> String {
    let text = path.to_string_lossy();
    text.strip_prefix(r"\\?\").unwrap_or(&text).to_string()
}
fn package_at(path: &Path) -> Option<Value> {
    let file = path.join("package.json");
    if fs::metadata(&file).ok()?.len() > 1024 * 1024 {
        return None;
    }
    serde_json::from_slice(&fs::read(file).ok()?).ok()
}
fn detect(path: &Path) -> (Vec<String>, Vec<String>) {
    let mut stack = Vec::new();
    let mut scripts = Vec::new();
    if let Some(pkg) = package_at(path) {
        stack.push("Node.js".into());
        for (name, label) in [
            ("react", "React"),
            ("next", "Next.js"),
            ("vue", "Vue"),
            ("vite", "Vite"),
            ("typescript", "TypeScript"),
            ("@tauri-apps/api", "Tauri"),
        ] {
            if pkg["dependencies"].get(name).is_some() || pkg["devDependencies"].get(name).is_some()
            {
                stack.push(label.into());
            }
        }
        if let Some(obj) = pkg["scripts"].as_object() {
            scripts = obj.keys().take(40).cloned().collect();
        }
    }
    if path.join("Cargo.toml").is_file() {
        stack.push("Rust".into());
    }
    if path.join("pyproject.toml").is_file() || path.join("requirements.txt").is_file() {
        stack.push("Python".into());
    }
    if path.join("pubspec.yaml").is_file() {
        stack.push("Flutter".into());
    }
    if path.join("composer.json").is_file() {
        stack.push("PHP".into());
    }
    if path.join("go.mod").is_file() {
        stack.push("Go".into());
    }
    if path.join("ProjectSettings").is_dir() && path.join("Assets").is_dir() {
        stack.push("Unity".into());
    }
    if let Ok(entries) = fs::read_dir(path) {
        for item in entries.take(200).flatten() {
            if matches!(
                item.path().extension().and_then(|x| x.to_str()),
                Some("csproj" | "sln")
            ) {
                stack.push(".NET".into());
                break;
            }
        }
    }
    (stack, scripts)
}

pub fn scan(root: &Path, root_id: &str) -> Result<(Vec<Project>, Vec<String>)> {
    let root = fs::canonicalize(root).map_err(|_| "Klasör bulunamadı veya erişilemiyor.")?;
    if !root.is_dir() {
        return Err("Bir proje klasörü seç.".into());
    }
    let mut pending = vec![(root, 0u8)];
    let mut found = Vec::new();
    let mut warnings = Vec::new();
    let mut visited = 0;
    while let Some((path, depth)) = pending.pop() {
        visited += 1;
        if visited > 5000 || found.len() >= 300 {
            warnings.push("Tarama sınırına ulaşıldı; daha küçük bir klasör seçebilirsin.".into());
            break;
        }
        let (stack, scripts) = detect(&path);
        if !stack.is_empty() {
            let text = path_text(&path);
            found.push(Project {
                id: path_id(&text),
                root_id: root_id.into(),
                name: path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                path: text,
                stack,
                scripts,
                favorite: false,
            });
            continue; // Treat a monorepo as one project; do not index dependencies inside it.
        }
        if depth >= 4 {
            continue;
        }
        match fs::read_dir(&path) {
            Ok(entries) => {
                for item in entries.flatten() {
                    let name = item.file_name().to_string_lossy().to_string();
                    if name.starts_with('.')
                        || [
                            "node_modules",
                            "target",
                            "dist",
                            "build",
                            "vendor",
                            "Library",
                            "Temp",
                        ]
                        .contains(&name.as_str())
                    {
                        continue;
                    }
                    if item
                        .file_type()
                        .map(|t| t.is_dir() && !t.is_symlink())
                        .unwrap_or(false)
                    {
                        pending.push((item.path(), depth + 1));
                    }
                }
            }
            Err(_) => warnings.push(format!("Okunamayan klasör: {}", path_text(&path))),
        }
    }
    found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok((found, warnings))
}

fn git_output(path: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let mut cmd = Command::new("git");
    cmd.args(["-c", "core.fsmonitor=false", "-c", "core.hooksPath=", "-C"])
        .arg(path)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let output = cmd
        .output()
        .map_err(|_| "Git bulunamadı. Git kurulumunu ve PATH ayarını kontrol et.")?;
    if !output.status.success() {
        return Err(
            "Bu klasörde kullanılabilir bir Git deposu yok veya Git erişimi reddetti.".into(),
        );
    }
    Ok(output.stdout)
}
pub fn git_info(path: &Path) -> GitInfo {
    if !path.join(".git").exists() {
        return GitInfo {
            error: Some("Bu proje bir Git deposu değil.".into()),
            ..Default::default()
        };
    }
    let status = match git_output(
        path,
        &["status", "--porcelain=v1", "-z", "--untracked-files=normal"],
    ) {
        Ok(value) => value,
        Err(error) => {
            return GitInfo {
                error: Some(error),
                ..Default::default()
            }
        }
    };
    let branch = git_output(path, &["symbolic-ref", "--quiet", "--short", "HEAD"])
        .or_else(|_| git_output(path, &["rev-parse", "--short", "HEAD"]))
        .unwrap_or_default();
    let mut changes = Vec::new();
    let mut parts = status.split(|x| *x == 0);
    while let Some(part) = parts.next() {
        if part.len() < 4 {
            continue;
        }
        changes.push(GitChange {
            status: String::from_utf8_lossy(&part[..2]).to_string(),
            path: String::from_utf8_lossy(&part[3..]).to_string(),
        });
        if part[..2].contains(&b'R') || part[..2].contains(&b'C') {
            parts.next();
        }
        if changes.len() >= 200 {
            break;
        }
    }
    let log = git_output(path, &["log", "-5", "--format=%h%x1f%s%x1f%ct"]).unwrap_or_default();
    let commits = String::from_utf8_lossy(&log)
        .lines()
        .filter_map(|line| {
            let p: Vec<_> = line.split('\x1f').collect();
            if p.len() != 3 {
                return None;
            }
            Some(json!({"hash":p[0],"subject":p[1],"time":p[2].parse::<u64>().unwrap_or(0)}))
        })
        .collect();
    GitInfo {
        available: true,
        branch: String::from_utf8_lossy(&branch).trim().into(),
        changes,
        commits,
        error: None,
    }
}

pub struct Board {
    pub center: processes::Center,
    pub data_path: PathBuf,
    pub library: Library,
}
impl Board {
    pub fn open(data_path: PathBuf) -> Result<Self> {
        fs::create_dir_all(&data_path).map_err(|e| e.to_string())?;
        let file = data_path.join("library.json");
        let library = if file.exists() {
            serde_json::from_slice(&fs::read(file).map_err(|e| e.to_string())?)
                .map_err(|_| "Kütüphane kaydı okunamadı; dosyanı yedekleyip onar.")?
        } else {
            Library::default()
        };
        let center = processes::Center::open(data_path.join("services.json"))?;
        Ok(Self {
            data_path,
            library,
            center,
        })
    }
    pub fn save(&self) -> Result<()> {
        let raw = serde_json::to_vec_pretty(&self.library).map_err(|e| e.to_string())?;
        let temp = self.data_path.join("library.json.tmp");
        fs::write(&temp, raw).map_err(|e| e.to_string())?;
        // Windows rename replaces a file through MoveFileEx in Rust's standard library.
        fs::rename(temp, self.data_path.join("library.json")).map_err(|e| e.to_string())
    }
    pub fn import_root(&mut self, raw: &str) -> Result<Value> {
        let path = fs::canonicalize(raw).map_err(|_| "Klasör bulunamadı.")?;
        let text = path_text(&path);
        let id = path_id(&text);
        if !self.library.roots.iter().any(|r| r.id == id) {
            if self.library.roots.len() >= 20 {
                return Err("En fazla 20 kaynak klasör eklenebilir.".into());
            }
            if !path.is_dir() {
                return Err("Bir klasör seç.".into());
            }
            self.library.roots.push(Root {
                id: id.clone(),
                path: text,
                scanned_at: now(),
            });
        }
        self.refresh_root(&id)
    }
    pub fn refresh_root(&mut self, id: &str) -> Result<Value> {
        let root = self
            .library
            .roots
            .iter()
            .find(|r| r.id == id)
            .ok_or("Kaynak bulunamadı.")?
            .clone();
        let (mut projects, warnings) = scan(Path::new(&root.path), id)?;
        let favorites: HashSet<_> = self
            .library
            .projects
            .iter()
            .filter(|p| p.favorite)
            .map(|p| p.id.clone())
            .collect();
        for p in &mut projects {
            p.favorite = favorites.contains(&p.id);
        }
        self.library.projects.retain(|p| p.root_id != id);
        let existing: HashSet<_> = self.library.projects.iter().map(|p| p.id.clone()).collect();
        self.library
            .projects
            .extend(projects.into_iter().filter(|p| !existing.contains(&p.id)));
        if let Some(r) = self.library.roots.iter_mut().find(|r| r.id == id) {
            r.scanned_at = now();
        }
        self.save()?;
        Ok(json!({"warnings":warnings}))
    }
    pub fn project(&self, id: &str) -> Result<&Project> {
        self.library
            .projects
            .iter()
            .find(|p| p.id == id)
            .ok_or("Proje bulunamadı.".into())
    }
    pub fn dispatch(&mut self, action: &str, args: Value) -> Result<Value> {
        let id = args["id"].as_str().unwrap_or("");
        match action {
            "snapshot" => Ok(json!({"roots":self.library.roots,"projects":self.library.projects})),
            "add_root" => self.import_root(args["path"].as_str().ok_or("Klasör yolu gerekli.")?),
            "scan_root" => self.refresh_root(id),
            "favorite" => {
                let p = self
                    .library
                    .projects
                    .iter_mut()
                    .find(|p| p.id == id)
                    .ok_or("Proje bulunamadı.")?;
                p.favorite = args["value"].as_bool().ok_or("Favori değeri gerekli.")?;
                self.save()?;
                Ok(json!(true))
            }
            "remove_root" => {
                self.library.projects.retain(|p| p.root_id != id);
                self.library.roots.retain(|r| r.id != id);
                self.save()?;
                Ok(json!(true))
            }
            "git_info" => {
                Ok(serde_json::to_value(git_info(Path::new(&self.project(id)?.path))).unwrap())
            }
            "services" => Ok(self.center.snapshot()),
            "service_add" => self.center.add(&args, &self.library.projects),
            "service_start" => self.center.start(id, &self.library.projects),
            "service_stop" => self.center.stop(id),
            "service_delete" => self.center.delete(id),
            "service_logs" => self.center.logs(id),
            "workspace_add" => self.center.workspace_add(&args),
            "workspace_start" => self.center.workspace_start(id, &self.library.projects),
            "workspace_stop" => self.center.workspace_stop(id),
            "workspace_delete" => self.center.workspace_delete(id),
            "port_check" => {
                let port = args["port"]
                    .as_u64()
                    .filter(|p| *p > 0 && *p <= 65535)
                    .ok_or("Geçerli port gerekli.")? as u16;
                Ok(json!({"port":port,"busy":processes::port_busy(port)}))
            }
            _ => Err("Bilinmeyen işlem.".into()),
        }
    }
}
