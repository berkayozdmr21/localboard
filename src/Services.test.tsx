import { afterEach, describe, it, expect, vi } from "vitest";
import {
  render,
  screen,
  fireEvent,
  waitFor,
  cleanup,
} from "@testing-library/react";
import Services, { parseArguments } from "./Services";
import { dispatch } from "./bridge";
vi.mock("./bridge", () => ({ dispatch: vi.fn() }));
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
const projects = [
  {
    id: "p",
    rootId: "r",
    name: "Web",
    path: "C:/Web",
    stack: ["React"],
    scripts: ["dev"],
    favorite: false,
  },
];
describe("process controls", () => {
  it("clears previous service command and port when reopening the form", async () => {
    vi.mocked(dispatch).mockResolvedValue({ services: [], workspaces: [] });
    render(<Services projects={projects} />);
    fireEvent.click(screen.getByRole("button", { name: "Servis ekle" }));
    fireEvent.change(screen.getByLabelText("Program"), {
      target: { value: "python" },
    });
    fireEvent.change(screen.getByRole("spinbutton"), {
      target: { value: "8000" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Kapat" }));
    fireEvent.click(screen.getByRole("button", { name: "Servis ekle" }));
    expect((screen.getByLabelText("Program") as HTMLInputElement).value).toBe(
      "npm",
    );
    expect((screen.getByRole("spinbutton") as HTMLInputElement).value).toBe("");
  });
  it("accepts structured args and rejects ambiguous input", () => {
    expect(parseArguments('["-m","uvicorn","main:app"]')).toEqual([
      "-m",
      "uvicorn",
      "main:app",
    ]);
    expect(() => parseArguments("npm run dev")).toThrow();
    expect(() => parseArguments("[1]")).toThrow();
  });
  it("requires a project before service configuration", async () => {
    vi.mocked(dispatch).mockResolvedValue({ services: [], workspaces: [] });
    render(<Services projects={[]} />);
    expect(
      screen
        .getByRole("button", { name: "Servis ekle" })
        .hasAttribute("disabled"),
    ).toBe(true);
    expect(screen.getByText("Önce bir proje ekle.")).toBeTruthy();
  });
  it("starts the selected registered service and surfaces port conflicts", async () => {
    vi.mocked(dispatch).mockImplementation(async (action) => {
      if (action === "service_start") throw Error("Port kullanımda");
      if (action === "service_logs") return [];
      return {
        services: [
          {
            id: "s",
            projectId: "p",
            name: "Frontend",
            executable: "npm",
            args: ["run", "dev"],
            port: 3000,
            status: "stopped",
            pid: null,
            exitCode: null,
          },
        ],
        workspaces: [],
      };
    });
    render(<Services projects={projects} />);
    fireEvent.click(
      await screen.findByRole("button", { name: "Frontend başlat" }),
    );
    await waitFor(() =>
      expect(dispatch).toHaveBeenCalledWith("service_start", { id: "s" }),
    );
    expect(await screen.findByRole("alert")).toBeTruthy();
  });
});
