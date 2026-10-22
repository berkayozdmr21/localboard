import { useEffect, useState, useCallback } from "react";
import {
  Folder,
  FolderPlus,
  LayoutGrid,
  Star,
  Search,
  ArrowUpRight,
  GitBranch,
  GitCommitHorizontal,
  RefreshCw,
  X,
  ChevronRight,
  ArrowLeft,
  Check,
  Command,
  Layers,
  AlertCircle,
  HardDrive,
  Trash2,
  LoaderCircle,
} from "lucide-react";
import { dispatch, chooseRoot, native } from "./bridge";
import type { Snapshot, Project, GitInfo } from "./types";
export function filterProjects(
  projects: Project[],
  query: string,
  favorites: boolean,
) {
  const q = query.toLocaleLowerCase("tr");
  return projects.filter(
    (p) =>
      (!favorites || p.favorite) &&
      [p.name, p.path, ...p.stack]
        .join(" ")
        .toLocaleLowerCase("tr")
        .includes(q),
  );
}
const stackColor: Record<string, string> = {
  React: "blue",
  Rust: "terra",
  Python: "amber",
  Unity: "slate",
  Flutter: "blue",
  ".NET": "violet",
  "Node.js": "green",
  PHP: "violet",
};
function shortPath(path: string) {
  return path.replace(/\\/g, "/");
}
export default function App() {
  const [snapshot, setSnapshot] = useState<Snapshot>({
    roots: [],
    projects: [],
  });
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [query, setQuery] = useState("");
  const [view, setView] = useState("library");
  const [selected, setSelected] = useState<string | null>(null);
  const [git, setGit] = useState<GitInfo | null>(null);
  const [gitLoading, setGitLoading] = useState(false);
  const [dialog, setDialog] = useState(false);
  const [rootPath, setRootPath] = useState("");
  const [about, setAbout] = useState(false);
  const load = useCallback(async () => {
    try {
      setSnapshot(await dispatch<Snapshot>("snapshot"));
      setError("");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);
  useEffect(() => {
    void load();
  }, [load]);
  const project = snapshot.projects.find((p) => p.id === selected);
  const projects = filterProjects(
    snapshot.projects,
    query,
    view === "favorites",
  );
  useEffect(() => {
    let valid = true;
    if (!project) {
      setGit(null);
      return;
    }
    setGitLoading(true);
    setGit(null);
    dispatch<GitInfo>("git_info", { id: project.id })
      .then((x) => {
        if (valid) setGit(x);
      })
      .catch((e) => {
        if (valid) setNotice(String(e));
      })
      .finally(() => {
        if (valid) setGitLoading(false);
      });
    return () => {
      valid = false;
    };
  }, [project?.id, snapshot]);
  useEffect(() => {
    function key(e: KeyboardEvent) {
      if (e.key === "Escape") {
        setDialog(false);
        setAbout(false);
        setSelected(null);
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "k") {
        e.preventDefault();
        document.getElementById("project-search")?.focus();
      }
    }
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, []);
  async function act(action: string, args: Record<string, unknown> = {}) {
    setBusy(true);
    try {
      const result = await dispatch<{ warnings?: string[] }>(action, args);
      await load();
      setNotice(result?.warnings?.join(" ") || "Kütüphane güncellendi.");
    } catch (e) {
      setNotice(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function addRoot() {
    if (native) {
      try {
        const path = await chooseRoot();
        if (path) await act("add_root", { path });
      } catch (e) {
        setNotice(String(e));
      }
    } else setDialog(true);
  }
  async function importPath(e: React.FormEvent) {
    e.preventDefault();
    await act("add_root", { path: rootPath });
    setDialog(false);
  }
  return (
    <div className="shell">
      <aside className="sidebar">
        <a
          className="brand"
          href="#"
          onClick={(e) => {
            e.preventDefault();
            setView("library");
          }}
        >
          <img src="/mark.svg" alt="" />
          <span>
            localboard<span className="brand-dot">.</span>
          </span>
        </a>
        <div className="workspace-label">KİŞİSEL ÇALIŞMA ALANI</div>
        <div className="workspace-name">
          <span className="workspace-icon">
            <Layers size={17} />
          </span>
          <div>
            Benim masam<small>Yerel çalışma alanı</small>
          </div>
          <ChevronRight size={15} />
        </div>
        <nav aria-label="Ana gezinme">
          <span className="nav-label">ÇALIŞMA MASASI</span>
          <button
            className={view === "library" ? "active" : ""}
            onClick={() => setView("library")}
          >
            <LayoutGrid size={17} />
            Proje kütüphanesi<span>{snapshot.projects.length}</span>
          </button>
          <button
            className={view === "favorites" ? "active" : ""}
            onClick={() => setView("favorites")}
          >
            <Star size={17} />
            Favoriler
            <span>{snapshot.projects.filter((p) => p.favorite).length}</span>
          </button>
          <button
            className={view === "roots" ? "active" : ""}
            onClick={() => setView("roots")}
          >
            <HardDrive size={17} />
            Kaynak klasörler<span>{snapshot.roots.length}</span>
          </button>
        </nav>
        <div className="sidebar-note">
          <span className="tiny-dot" /> Yalnızca bu bilgisayarda
          <p>
            Projelerin yerinde kalır.
            <br />
            Her şey senin kontrolünde.
          </p>
        </div>
        <button className="profile" onClick={() => setAbout(true)}>
          <span className="avatar">B</span>
          <div>
            Berkay Özdemir<small>LocalBoard / Masaüstü</small>
          </div>
          <Command size={15} />
        </button>
      </aside>
      <div className="workspace">
        <header className="topbar">
          <span>
            Çalışma alanım <ChevronRight size={13} />{" "}
            {view === "roots"
              ? "Kaynak klasörler"
              : view === "favorites"
                ? "Favoriler"
                : "Proje kütüphanesi"}
          </span>
          <div>
            <span className="local-pill">
              <span className="tiny-dot" /> LOCAL FIRST
            </span>
            <button
              className="icon-button"
              aria-label="Kütüphaneyi yenile"
              disabled={busy}
              onClick={() => void load()}
            >
              <RefreshCw size={16} />
            </button>
          </div>
        </header>
        <main>
          <section className="hero">
            <div>
              <div className="eyebrow">WORKSPACE / 01</div>
              <h1>Çalışma masası</h1>
              <p>Birlikte çalıştığın tüm projeler, tek bir masada.</p>
            </div>
            <button
              className="primary"
              onClick={() => void addRoot()}
              disabled={busy}
            >
              <FolderPlus size={17} /> Klasör ekle <span>↗</span>
            </button>
          </section>
          <div className="summary-grid">
            <div>
              <span>PROJE KÜTÜPHANESİ</span>
              <strong>
                {snapshot.projects.length.toString().padStart(2, "0")}
                <LayoutGrid size={20} />
              </strong>
              <small>Keşfedilen yerel proje</small>
            </div>
            <div>
              <span>KAYNAK KLASÖRLER</span>
              <strong>
                {snapshot.roots.length.toString().padStart(2, "0")}
                <Folder size={20} />
              </strong>
              <small>Yalnızca seçtiğin klasörler</small>
            </div>
            <div className="summary-accent">
              <span>DEPOLAMA</span>
              <strong>
                Bu cihaz
                <HardDrive size={20} />
              </strong>
              <small>Hesap yok. Bulut yok. Kaynak kodun sende.</small>
            </div>
          </div>
          {error && (
            <div role="alert" className="error-banner">
              <AlertCircle size={18} />
              <div>
                <strong>Çalışma alanına bağlanılamadı.</strong>
                <p>{error}</p>
                <button
                  onClick={() => {
                    setLoading(true);
                    void load();
                  }}
                >
                  Yeniden dene
                </button>
              </div>
            </div>
          )}
          {view === "roots" ? (
            <section className="roots-view">
              <div className="section-heading">
                <h2>Kaynak klasörler</h2>
                <span>{snapshot.roots.length} kaynak</span>
              </div>
              {snapshot.roots.length === 0 ? (
                <div className="empty">
                  <FolderPlus size={34} />
                  <h3>İlk klasörünle başlayalım.</h3>
                  <p>
                    Proje klasörünü veya projelerini tuttuğun üst klasörü ekle.
                  </p>
                  <button className="primary" onClick={() => void addRoot()}>
                    Klasör seç
                  </button>
                </div>
              ) : (
                snapshot.roots.map((root) => (
                  <article className="root-row" key={root.id}>
                    <Folder size={22} />
                    <div>
                      <strong>{shortPath(root.path)}</strong>
                      <small>
                        Son tarama:{" "}
                        {new Date(root.scannedAt * 1000).toLocaleString(
                          "tr-TR",
                        )}
                      </small>
                    </div>
                    <button
                      className="secondary"
                      disabled={busy}
                      onClick={() => void act("scan_root", { id: root.id })}
                    >
                      <RefreshCw size={14} /> Yeniden tara
                    </button>
                    <button
                      className="icon-button"
                      aria-label="Kaynağı kütüphaneden kaldır"
                      disabled={busy}
                      onClick={() => void act("remove_root", { id: root.id })}
                    >
                      <Trash2 size={15} />
                    </button>
                  </article>
                ))
              )}
            </section>
          ) : (
            <>
              <div className="collection-toolbar">
                <div className="collection-tabs">
                  <button
                    className={view === "library" ? "selected" : ""}
                    onClick={() => setView("library")}
                  >
                    Tüm projeler <span>{snapshot.projects.length}</span>
                  </button>
                  <button
                    className={view === "favorites" ? "selected" : ""}
                    onClick={() => setView("favorites")}
                  >
                    Favoriler
                  </button>
                </div>
                <label className="search">
                  <Search size={16} />
                  <input
                    id="project-search"
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                    placeholder="Proje veya teknoloji ara"
                    aria-label="Proje ara"
                  />
                  <kbd>Ctrl K</kbd>
                </label>
              </div>
              <div className="section-heading">
                <h2>
                  {view === "favorites"
                    ? "Favori projeler"
                    : "Projeler"}
                </h2>
                <span>{projects.length} proje</span>
              </div>
              {loading ? (
                <div className="empty">
                  <LoaderCircle className="spin" size={26} />
                  <p>Kütüphane hazırlanıyor…</p>
                </div>
              ) : projects.length === 0 ? (
                <div className="empty">
                  <div className="empty-symbol">
                    <FolderPlus size={30} />
                  </div>
                  <h3>
                    {query
                      ? "Aradığın proje bulunamadı."
                      : view === "favorites"
                        ? "Favorilerine yer ayırdık."
                        : "Masanda yeni fikirlere yer var."}
                  </h3>
                  <p>
                    {query
                      ? "Proje adı veya teknolojiyle tekrar ara."
                      : view === "favorites"
                        ? "Proje kartındaki yıldızla favorilerine ekleyebilirsin."
                        : "Bir klasör seç; projelerini ve teknoloji yığınlarını birlikte keşfedelim."}
                  </p>
                  {!query && view !== "favorites" && (
                    <button
                      className="secondary"
                      onClick={() => void addRoot()}
                    >
                      İlk klasörünü ekle <ArrowUpRight size={15} />
                    </button>
                  )}
                </div>
              ) : (
                <div
                  className={"project-grid " + (project ? "has-inspector" : "")}
                >
                  {projects.map((p) => (
                    <article
                      className={
                        "project-card " +
                        (p.id === selected ? "is-selected" : "")
                      }
                      key={p.id}
                    >
                      <div className="card-heading">
                        <span
                          className={
                            "project-icon " +
                            (stackColor[
                              p.stack.find((x) => x in stackColor) || ""
                            ] || "green")
                          }
                        >
                          <Folder size={22} />
                        </span>
                        <button
                          className={
                            "star-button " + (p.favorite ? "is-favorite" : "")
                          }
                          aria-label={
                            p.favorite
                              ? `${p.name} favorilerden çıkar`
                              : `${p.name} favorilere ekle`
                          }
                          disabled={busy}
                          onClick={() =>
                            void act("favorite", {
                              id: p.id,
                              value: !p.favorite,
                            })
                          }
                        >
                          <Star
                            size={17}
                            fill={p.favorite ? "currentColor" : "none"}
                          />
                        </button>
                      </div>
                      <button
                        className="card-open"
                        onClick={() => setSelected(p.id)}
                      >
                        <h3>{p.name}</h3>
                        <p title={p.path}>{shortPath(p.path)}</p>
                      </button>
                      <div className="stack-tags">
                        {p.stack.slice(0, 4).map((s) => (
                          <span key={s}>{s}</span>
                        ))}
                      </div>
                      <div className="card-bottom">
                        <span>
                          <span className="tiny-dot" /> Yerel proje
                        </span>
                        <button onClick={() => setSelected(p.id)}>
                          İncele <ArrowUpRight size={14} />
                        </button>
                      </div>
                    </article>
                  ))}
                </div>
              )}
            </>
          )}
          <footer>
            <span>
              <span className="tiny-dot" />{" "}
              {busy
                ? "Kütüphane güncelleniyor…"
                : "Yerel kütüphane hazır"}
            </span>
            <span>
              LOCALBOARD <b>v0.1</b>
            </span>
          </footer>
        </main>
      </div>
      {project && (
        <aside className="inspector">
          <div className="inspector-top">
            <span>PROJEYE YAKINDAN BAK</span>
            <button
              className="icon-button"
              aria-label="Proje detayını kapat"
              onClick={() => setSelected(null)}
            >
              <X size={18} />
            </button>
          </div>
          <span className="detail-icon">
            <Folder size={27} />
          </span>
          <h2>{project.name}</h2>
          <p className="detail-path">{shortPath(project.path)}</p>
          <div className="stack-tags">
            {project.stack.map((s) => (
              <span key={s}>{s}</span>
            ))}
          </div>
          <div className="inspector-section">
            <h3>
              <GitBranch size={16} /> Git durumu
            </h3>
            {gitLoading ? (
              <p>Git bilgileri okunuyor…</p>
            ) : git?.available ? (
              <>
                <div className="branch-line">
                  <GitBranch size={14} />
                  <strong>{git.branch || "Henüz commit yok"}</strong>
                  <span>{git.changes.length ? "Değişiklik var" : "Temiz"}</span>
                </div>
                <h4>
                  Değişen dosyalar <span>{git.changes.length}</span>
                </h4>
                {git.changes.length === 0 ? (
                  <p className="quiet">
                    <Check size={15} /> Çalışma ağacı temiz.
                  </p>
                ) : (
                  <div className="changes-list">
                    {git.changes.map((c, i) => (
                      <div key={i}>
                        <code>{c.status.trim()}</code>
                        <span title={c.path}>{c.path}</span>
                      </div>
                    ))}
                  </div>
                )}
                <h4>Son commit’ler</h4>
                {git.commits.length === 0 ? (
                  <p>Henüz commit yok.</p>
                ) : (
                  git.commits.map((c) => (
                    <div className="commit-row" key={c.hash}>
                      <GitCommitHorizontal size={15} />
                      <div>
                        <strong>{c.subject}</strong>
                        <small>
                          {c.hash} ·{" "}
                          {new Date(c.time * 1000).toLocaleDateString("tr-TR")}
                        </small>
                      </div>
                    </div>
                  ))
                )}
              </>
            ) : (
              <p className="quiet">{git?.error || "Git bilgisi yok."}</p>
            )}
          </div>
          <div className="inspector-section">
            <h3>
              <Command size={16} /> Proje scriptleri
            </h3>
            {project.scripts.length ? (
              <div className="script-names">
                {project.scripts.map((s) => (
                  <code key={s}>npm run {s}</code>
                ))}
              </div>
            ) : (
              <p className="quiet">package.json içinde script bulunamadı.</p>
            )}
          </div>
        </aside>
      )}
      {notice && (
        <div className="toast" role="status">
          <Check size={17} />
          <span>{notice}</span>
          <button aria-label="Bildirimi kapat" onClick={() => setNotice("")}>
            <X size={15} />
          </button>
        </div>
      )}
      {dialog && (
        <div className="modal-backdrop">
          <form className="modal" onSubmit={importPath}>
            <button
              type="button"
              className="modal-close icon-button"
              aria-label="Kapat"
              onClick={() => setDialog(false)}
            >
              <X size={18} />
            </button>
            <span className="eyebrow">YENİ BİR KAYNAK</span>
            <h2>Projelerin nerede?</h2>
            <p>
              Tarayıcı önizlemesinde tam klasör yolunu gir. Masaüstü
              uygulamasında yerel klasör seçici açılır.
            </p>
            <label>
              Klasör yolu
              <input
                autoFocus
                value={rootPath}
                onChange={(e) => setRootPath(e.target.value)}
                placeholder="C:\Users\...\Projects"
                required
              />
            </label>
            <p className="field-note">
              En fazla dört alt düzey taranır. Bağımlılıklar ve gizli klasörler
              atlanır; kaynak dosyaların değiştirilmez.
            </p>
            <button className="primary" disabled={busy}>
              {busy ? (
                <LoaderCircle className="spin" size={16} />
              ) : (
                <FolderPlus size={16} />
              )}{" "}
              Klasörü tara
            </button>
          </form>
        </div>
      )}
      {about && (
        <div className="modal-backdrop">
          <section
            className="modal"
            role="dialog"
            aria-label="LocalBoard hakkında"
          >
            <button
              className="modal-close icon-button"
              aria-label="Kapat"
              onClick={() => setAbout(false)}
            >
              <X size={18} />
            </button>
            <img className="about-logo" src="/mark.svg" alt="LocalBoard" />
            <h2>Sana ait bir çalışma masası.</h2>
            <p>
              LocalBoard, Berkay Özdemir tarafından geliştirilen yerel proje
              kütüphanesidir. Kaynak kodun ve kütüphane verilerin bu
              bilgisayarda saklanır.
            </p>
            <p className="field-note">
              Tauri · React · TypeScript · Rust
              <br />
              v0.1 · Proje kütüphanesi
            </p>
          </section>
        </div>
      )}
    </div>
  );
}
