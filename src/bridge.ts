export const native = "__TAURI_INTERNALS__" in window;
export async function dispatch<T>(
  action: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  if (native) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>("dispatch", { action, args });
  }
  const response = await fetch("/api/dispatch", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Localboard-Client": "studio",
    },
    body: JSON.stringify({ action, args }),
  });
  let value;
  try {
    value = await response.json();
  } catch {
    throw Error(
      "Yerel bağlantı kurulamadı. LocalBoard companion servisinin açık olduğundan emin ol.",
    );
  }
  if (!response.ok) throw Error(value.error || "İşlem tamamlanamadı.");
  return value.data as T;
}
export async function chooseRoot(): Promise<string | null> {
  if (!native) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string | null>("choose_root");
}
