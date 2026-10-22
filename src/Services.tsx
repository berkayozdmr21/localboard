import { useEffect, useState, useCallback } from "react";
import {
  Play,
  Square,
  Plus,
  Terminal,
  Trash2,
  X,
  Layers,
  Radio,
  ArrowUpRight,
  Check,
  AlertCircle,
  ChevronRight,
  Copy,
  LoaderCircle,
} from "lucide-react";
import { dispatch } from "./bridge";
import type { Project } from "./types";
type Service = {
  id: string;
  projectId: string;
  name: string;
  executable: string;
  args: string[];
  port: number | null;
  status: "running" | "exited" | "stopped";
  pid: number | null;
  exitCode: number | null;
};
type Workspace = { id: string; name: string; serviceIds: string[] };
type State = { services: Service[]; workspaces: Workspace[] };
type Log = { seq: number; time: number; stream: string; text: string };
export function parseArguments(raw: string): string[] {
  const value: unknown = JSON.parse(raw);
  if (!Array.isArray(value) || value.some((v) => typeof v !== "string"))
    throw Error("Argümanları JSON metin dizisi olarak gir.");
  return value;
}
export default function Services({ projects }: { projects: Project[] }) {
  const [state, setState] = useState<State>({ services: [], workspaces: [] });
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [logs, setLogs] = useState<Log[]>([]);
  const [modal, setModal] = useState<"service" | "workspace" | null>(null);
  const [name, setName] = useState("");
  const [projectId, setProjectId] = useState(projects[0]?.id || "");
  const [executable, setExecutable] = useState("npm");
  const [args, setArgs] = useState('["run", "dev"]');
  const [port, setPort] = useState("");
  const [checked, setChecked] = useState<string[]>([]);
  const [portResult, setPortResult] = useState("");
  const refresh = useCallback(async () => {
    setState(await dispatch<State>("services"));
  }, []);
  useEffect(() => {
    let valid = true;
    async function poll() {
      try {
        const data = await dispatch<State>("services");
        if (valid) setState(data);
      } catch (e) {
        if (valid) setError(String(e));
      }
    }
    void poll();
    const timer = setInterval(() => void poll(), 1500);
    return () => {
      valid = false;
      clearInterval(timer);
    };
  }, []);
  useEffect(() => {
    let valid = true;
    setLogs([]);
    if (!selected) return;
    async function poll() {
      try {
        const data = await dispatch<Log[]>("service_logs", { id: selected });
        if (valid) setLogs(data);
      } catch (e) {
        if (valid) setError(String(e));
      }
    }
    void poll();
    const timer = setInterval(() => void poll(), 1000);
    return () => {
      valid = false;
      clearInterval(timer);
    };
  }, [selected]);
  useEffect(() => {
    function key(e: KeyboardEvent) {
      if (e.key === "Escape") setModal(null);
    }
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, []);
  const active = state.services.find((s) => s.id === selected);
  const project = projects.find((p) => p.id === projectId);
  const running = state.services.filter((s) => s.status === "running").length;
  async function action(command: string, data: Record<string, unknown> = {}) {
    setBusy(true);
    setError("");
    try {
      await dispatch(command, data);
      await refresh();
      return true;
    } catch (e) {
      setError(String(e));
      return false;
    } finally {
      setBusy(false);
    }
  }
  function open(kind: "service" | "workspace") {
    setModal(kind);
    setName("");
    setPortResult("");
    setChecked([]);
    if (!projectId) setProjectId(projects[0]?.id || "");
  }
  async function save(e: React.FormEvent) {
    e.preventDefault();
    try {
      let ok = false;
      if (modal === "service") {
        ok = await action("service_add", {
          name,
          projectId,
          executable,
          args: parseArguments(args),
          port: port ? Number(port) : null,
        });
      } else ok = await action("workspace_add", { name, serviceIds: checked });
      if (ok) setModal(null);
    } catch (e) {
      setError(String(e));
    }
  }
  async function checkPort() {
    try {
      const value = await dispatch<{ busy: boolean }>("port_check", {
        port: Number(port),
      });
      setPortResult(value.busy ? "Port kullanımda" : "Port boş");
    } catch (e) {
      setPortResult(String(e));
    }
  }
  return (
    <section className="service-center">
      <div className="service-heading">
        <div>
          <span className="eyebrow">OPERATIONS / LOCAL</span>
          <h2>Kontrol merkezi</h2>
          <p>Servisleri başlat. Çıktıyı izle. Kontrol sende.</p>
        </div>
        <button
          className="primary"
          disabled={!projects.length || busy}
          onClick={() => open("service")}
        >
          <Plus size={16} /> Servis ekle
        </button>
      </div>
      <div className="operations-strip">
        <span>
          <i className={running ? "live-light" : ""} />
          <strong>{running}</strong> çalışan servis
        </span>
        <span>
          <Terminal size={14} />
          {state.services.length} tanımlı komut
        </span>
        <span>
          <Layers size={14} />
          {state.workspaces.length} çalışma alanı
        </span>
        <span className="strip-right">MANUAL START ONLY</span>
      </div>
      {error && (
        <div className="error-banner" role="alert">
          <AlertCircle size={17} />
          <span>{error}</span>
          <button aria-label="Hatayı kapat" onClick={() => setError("")}>
            <X size={15} />
          </button>
        </div>
      )}
      {!projects.length && (
        <div className="empty">
          <Terminal size={30} />
          <h3>Önce bir proje ekle.</h3>
          <p>
            Proje kütüphanesinde bir kaynak klasör seç; sonra servis komutlarını
            burada tanımlayabilirsin.
          </p>
        </div>
      )}
      <div className="operations-layout">
        <div className="service-list">
          <div className="section-heading">
            <h3>
              Servisler{" "}
              <span>{state.services.length.toString().padStart(2, "0")}</span>
            </h3>
            <span>PROGRAM / DURUM</span>
          </div>
          {state.services.length === 0 && projects.length > 0 ? (
            <div className="service-empty">
              <Terminal size={25} />
              <h3>Henüz servis yok</h3>
              <p>
                Örneğin frontend için <code>npm run dev</code>, backend için{" "}
                <code>python -m uvicorn main:app</code> tanımlayabilirsin.
              </p>
              <button className="secondary" onClick={() => open("service")}>
                Komut tanımla <ArrowUpRight size={14} />
              </button>
            </div>
          ) : (
            state.services.map((s, index) => (
              <article
                className={
                  "service-row " + (selected === s.id ? "selected" : "")
                }
                key={s.id}
              >
                <button
                  className="service-select"
                  onClick={() => setSelected(s.id)}
                >
                  <span className="service-index">
                    {(index + 1).toString().padStart(2, "0")}
                  </span>
                  <div>
                    <strong>{s.name}</strong>
                    <small>
                      {projects.find((p) => p.id === s.projectId)?.name ||
                        "Proje kaldırıldı"}{" "}
                      <span>/</span> {s.executable} {s.args.join(" ")}
                    </small>
                  </div>
                  <ChevronRight size={15} />
                </button>
                <div className="service-row-bottom">
                  <span className={"process-badge " + s.status}>
                    <i />
                    {s.status === "running"
                      ? `Çalışıyor · PID ${s.pid}`
                      : s.status === "exited"
                        ? `Sonlandı · ${s.exitCode ?? "sinyal"}`
                        : "Durduruldu"}
                  </span>
                  {s.port && <span className="port-tag">:{s.port}</span>}
                  <div>
                    {s.status === "running" ? (
                      <button
                        className="stop-control"
                        disabled={busy}
                        aria-label={`${s.name} durdur`}
                        onClick={() =>
                          void action("service_stop", { id: s.id })
                        }
                      >
                        <Square size={11} fill="currentColor" />
                        Durdur
                      </button>
                    ) : (
                      <button
                        className="start-control"
                        disabled={busy}
                        aria-label={`${s.name} başlat`}
                        onClick={() => {
                          setSelected(s.id);
                          void action("service_start", { id: s.id });
                        }}
                      >
                        <Play size={12} fill="currentColor" />
                        Başlat
                      </button>
                    )}
                    <button
                      className="icon-button"
                      disabled={busy || s.status === "running"}
                      aria-label={`${s.name} servisini kaldır`}
                      onClick={() =>
                        void action("service_delete", { id: s.id })
                      }
                    >
                      <Trash2 size={13} />
                    </button>
                  </div>
                </div>
              </article>
            ))
          )}
          <div className="workspace-section">
            <div className="section-heading">
              <h3>Çalışma alanları</h3>
              <button
                disabled={busy || !state.services.length}
                onClick={() => open("workspace")}
              >
                <Plus size={13} /> Alan ekle
              </button>
            </div>
            {!state.workspaces.length ? (
              <p className="muted">
                Frontend ve backend servislerini birlikte başlatmak için bir
                alan oluştur.
              </p>
            ) : (
              state.workspaces.map((w) => (
                <article className="group-card" key={w.id}>
                  <div>
                    <span className="group-symbol">
                      <Layers size={17} />
                    </span>
                    <div>
                      <strong>{w.name}</strong>
                      <small>
                        {w.serviceIds.length} servis ·{" "}
                        {
                          w.serviceIds.filter((id) =>
                            state.services.some(
                              (s) => s.id === id && s.status === "running",
                            ),
                          ).length
                        }{" "}
                        çalışıyor
                      </small>
                    </div>
                  </div>
                  <div className="group-actions">
                    <button
                      disabled={busy}
                      onClick={() =>
                        void action("workspace_start", { id: w.id })
                      }
                    >
                      <Play size={12} />
                      Tümünü başlat
                    </button>
                    <button
                      disabled={busy}
                      onClick={() =>
                        void action("workspace_stop", { id: w.id })
                      }
                    >
                      <Square size={11} />
                      Durdur
                    </button>
                    <button
                      className="icon-button"
                      disabled={busy}
                      aria-label={`${w.name} alanını kaldır`}
                      onClick={() =>
                        void action("workspace_delete", { id: w.id })
                      }
                    >
                      <Trash2 size={12} />
                    </button>
                  </div>
                </article>
              ))
            )}
          </div>
        </div>
        <div className="console">
          <div className="console-header">
            <span>
              <Terminal size={15} />
              {active?.name || "Process console"}
            </span>
            <span className="console-dots">
              <i />
              <i />
              <i />
            </span>
          </div>
          <div className="console-meta">
            <span>
              {active
                ? `${active.executable} ${active.args.join(" ")}`
                : "Servis seçimini bekliyor"}
            </span>
            <button
              disabled={!logs.length}
              aria-label="Logları kopyala"
              onClick={() => {
                void navigator.clipboard
                  .writeText(
                    logs.map((l) => `[${l.stream}] ${l.text}`).join("\n"),
                  )
                  .catch((e) => setError(String(e)));
              }}
            >
              <Copy size={13} />
            </button>
          </div>
          <div
            className="console-output"
            role="log"
            aria-label="Servis çıktısı"
          >
            {logs.length ? (
              logs.map((l) => (
                <div className={"log-line " + l.stream} key={l.seq}>
                  <time>
                    {new Date(l.time * 1000).toLocaleTimeString("tr-TR")}
                  </time>
                  <pre>{l.text || " "}</pre>
                </div>
              ))
            ) : (
              <div className="console-placeholder">
                <span>›_</span>
                <p>
                  {active
                    ? "Servis henüz çıktı üretmedi."
                    : "Çıktısını görmek için bir servis seç."}
                </p>
                <small>stdout + stderr · son 600 kayıt</small>
              </div>
            )}
          </div>
          <div className="console-footer">
            <span>
              <i className={active?.status === "running" ? "live-light" : ""} />
              {active?.status === "running" ? "PROCESS RUNNING" : "IDLE"}
            </span>
            <span>{logs.length} satır</span>
          </div>
        </div>
      </div>
      {modal && (
        <div className="modal-backdrop">
          <form
            className="modal service-modal"
            role="dialog"
            aria-modal="true"
            aria-label={
              modal === "service" ? "Servis ekle" : "Çalışma alanı ekle"
            }
            onSubmit={save}
          >
            <button
              type="button"
              className="modal-close icon-button"
              aria-label="Kapat"
              onClick={() => setModal(null)}
            >
              <X size={18} />
            </button>
            <span className="eyebrow">
              {modal === "service" ? "NEW PROCESS" : "NEW WORKSPACE"}
            </span>
            <h2>
              {modal === "service"
                ? "Bir komut, bir servis."
                : "Birlikte çalışsınlar."}
            </h2>
            <label>
              {modal === "service" ? "Servis adı" : "Alan adı"}
              <input
                required
                maxLength={100}
                value={name}
                onChange={(e) => setName(e.target.value)}
                autoFocus
                placeholder={
                  modal === "service"
                    ? "Frontend geliştirme"
                    : "Ürün geliştirme"
                }
              />
            </label>
            {modal === "service" ? (
              <>
                <label>
                  Proje
                  <select
                    value={projectId}
                    onChange={(e) => setProjectId(e.target.value)}
                    required
                  >
                    {projects.map((p) => (
                      <option value={p.id} key={p.id}>
                        {p.name}
                      </option>
                    ))}
                  </select>
                </label>
                {!!project?.scripts.length && (
                  <div className="script-picks">
                    <span>npm scriptleri</span>
                    {project.scripts.map((s) => (
                      <button
                        type="button"
                        key={s}
                        onClick={() => {
                          setExecutable("npm");
                          setArgs(JSON.stringify(["run", s]));
                          if (!name) setName(s);
                        }}
                      >
                        {s}
                      </button>
                    ))}
                  </div>
                )}
                <div className="command-fields">
                  <label>
                    Program
                    <input
                      required
                      value={executable}
                      onChange={(e) => setExecutable(e.target.value)}
                      placeholder="npm / python / node / program.exe"
                    />
                  </label>
                  <label>
                    Port (isteğe bağlı)
                    <div className="port-input">
                      <input
                        type="number"
                        min="1"
                        max="65535"
                        value={port}
                        onChange={(e) => {
                          setPort(e.target.value);
                          setPortResult("");
                        }}
                        placeholder="3000"
                      />
                      <button
                        type="button"
                        disabled={!port}
                        onClick={() => void checkPort()}
                      >
                        Kontrol
                      </button>
                    </div>
                  </label>
                </div>
                {portResult && <p className="port-result">{portResult}</p>}
                <label>
                  Argümanlar (JSON dizisi)
                  <input
                    required
                    value={args}
                    onChange={(e) => setArgs(e.target.value)}
                    spellCheck={false}
                  />
                </label>
                <div className="command-review">
                  <span>ÇALIŞTIRILACAK KOMUT</span>
                  <code>
                    {executable}{" "}
                    {(() => {
                      try {
                        return parseArguments(args)
                          .map((a) => (/\s/.test(a) ? JSON.stringify(a) : a))
                          .join(" ");
                      } catch {
                        return "— geçersiz argümanlar";
                      }
                    })()}
                  </code>
                  <small>{project?.path}</small>
                </div>
                <p className="field-note">
                  Kaydetmek komutu çalıştırmaz. Servis yalnızca Başlat
                  düğmesiyle, bu projenin klasöründe çalışır. Kendi güvendiğin
                  komutları kullan.
                </p>
              </>
            ) : (
              <>
                <p>Birlikte başlatılacak servisleri seç.</p>
                <div className="workspace-checkboxes">
                  {state.services.map((s) => (
                    <label key={s.id}>
                      <input
                        type="checkbox"
                        checked={checked.includes(s.id)}
                        onChange={(e) =>
                          setChecked(
                            e.target.checked
                              ? [...checked, s.id]
                              : checked.filter((id) => id !== s.id),
                          )
                        }
                      />
                      <span>
                        {s.name}
                        <small>
                          {s.executable} {s.args.join(" ")}
                        </small>
                      </span>
                    </label>
                  ))}
                </div>
                <p className="field-note">
                  Bir komut başlatılamazsa bu işlem sırasında başlatılan
                  servisler durdurulur. Zaten çalışanlar korunur.
                </p>
              </>
            )}
            {error && (
              <div className="error-banner" role="alert">
                {error}
              </div>
            )}
            <button
              className="primary"
              disabled={busy || (modal === "workspace" && !checked.length)}
            >
              {busy ? (
                <LoaderCircle className="spin" size={15} />
              ) : (
                <Plus size={15} />
              )}{" "}
              {modal === "service" ? "Servisi kaydet" : "Alanı oluştur"}
            </button>
          </form>
        </div>
      )}
    </section>
  );
}
