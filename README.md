<p align="center"><img src="public/mark.svg" width="68" alt="LocalBoard" /></p>
<h1 align="center">LocalBoard</h1>
<p align="center">A quiet home for your development projects.<br/><strong>Tauri · React · TypeScript · Rust</strong><br/>Built by Berkay Özdemir</p>

LocalBoard is a Windows desktop workspace for discovering development projects, inspecting Git changes and managing local services. Choose a folder, keep favorites close and bring frontend/backend processes together in named workspaces.

No account, API key or external backend is required. Source files stay in their original locations.

## Features

- **Project discovery:** Node.js, React, Next.js, Vue, Vite, TypeScript, Tauri, Rust, Python, Flutter, Go, PHP, .NET and Unity markers.
- **Persistent library:** search by name, path or technology; favorites survive rescans and restarts. Removing a source never deletes project files.
- **Git insights:** branch, changed file status and the latest five commits. LocalBoard never commits or pushes project code.
- **Explicit services:** review a program and its arguments, then start or stop it in the registered project directory.
- **Process console:** bounded stdout/stderr, process ID and exit code. Port checks reject occupied ports without stopping unrelated processes.
- **Named workspaces:** group several services, start/stop together, roll back newly started services if a launch fails.

## Run on Windows

Download `localboard-windows` from the latest successful [Windows build](https://github.com/berkayozdmr21/localboard/actions), unzip it and open `target/release/localboard.exe`.

Windows 10/11 and Microsoft Edge WebView2 are required. Node.js, Rust and Visual Studio are only needed to build from source. Git must be on PATH for repository inspection; service runtimes such as Node/Python must be installed for the commands you configure.

## Development

Use Node.js 24.15+ LTS, Rust stable and the [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/), including Visual Studio C++ tools and the Windows SDK.

```powershell
npm ci
npm run desktop
```

Build the portable executable:

```powershell
npm run desktop:build
```

The optional browser companion uses the **same Rust core**, not a mock API:

```powershell
npm run build
cargo run -p localboard-preview
# Open http://127.0.0.1:9140
```

For hot reload, keep the companion running and run `npm run dev` in another terminal. Open `http://127.0.0.1:1420`. `LOCALBOARD_PORT` changes the companion port; update the Vite proxy as well for hot reload. Use `127.0.0.1`, not `localhost`.

## Verification

```powershell
npm test
npm run build
cargo test -p localboard-core --locked
```

Windows CI runs frontend interaction tests and native tests, then builds both the companion and Tauri executables. Tests use real temporary project directories and real child processes.

## Architecture

```text
src/                         React workspace and typed IPC bridge
crates/localboard-core/       Discovery, persistence, Git, process supervision
crates/localboard-preview/    Optional loopback HTTP adapter
src-tauri/                   Native window, folder picker and IPC
.github/workflows/           Windows tests and portable builds
```

The native app uses its per-user Tauri app-data directory. The companion uses `.localboard/` in the working directory, or `LOCALBOARD_DATA`. These stores are independent. Local paths, favorites and service configuration remain on this device and are excluded from Git. Runtime logs are memory-only.

## Operating boundaries

- Discovery stops after four levels, 5,000 visited directories or 300 projects per source. Hidden/dependency directories and child symlinks are skipped. Recognized monorepos count as one project.
- Git changes display at most 200 entries. Technology markers do not prove a project is healthy or dependencies are installed.
- Services run only when explicitly started; adding/scanning a project executes nothing. Commands run with the current user's permissions and environment. Only configure code you trust.
- A running status means the process exists, not that its HTTP service is ready. Port checks use TCP on `127.0.0.1`; there is a normal race between checking and the application's own bind.
- On Windows, stopping a live owned process uses `taskkill /T` to stop its process tree. Normal app exit stops owned services. Forced app termination or a parent that exits before its detached children can leave descendants; this is not a Windows service manager.
- Logs retain the latest 600 chunks per service, capped at 4 KiB each. Restarting a service begins a fresh log buffer.
- The companion binds only to loopback and checks Host, Origin and the client header. It is for a trusted desktop, not public hosting. No telemetry or remote font loading.

## Feature PRs

1. [Project library and Git insights](https://github.com/berkayozdmr21/localboard/pull/1)
2. Process control center and named workspaces.

## Contact

[Berkay Özdemir on LinkedIn](https://www.linkedin.com/in/berkay-%C3%B6zdemir-7332b2248/) · [Email](mailto:berkay.oz3321@gmail.com)
