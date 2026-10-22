import { describe, it, expect, vi } from "vitest";
import {
  render,
  screen,
  fireEvent,
  waitFor,
  cleanup,
} from "@testing-library/react";
import { afterEach } from "vitest";
import App, { filterProjects } from "./App";
import { dispatch } from "./bridge";
vi.mock("./bridge", () => ({
  native: false,
  chooseRoot: vi.fn(),
  dispatch: vi.fn(),
}));
const project = {
  id: "p",
  rootId: "r",
  name: "Orbit Explorer",
  path: "C:/Projects/orbit",
  stack: ["React", "TypeScript"],
  scripts: ["dev"],
  favorite: false,
};
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
describe("project library", () => {
  it("filters real project names, paths, technology and favorites", () => {
    expect(filterProjects([project], "react", false)).toHaveLength(1);
    expect(filterProjects([project], "Projects/orbit", false)).toHaveLength(1);
    expect(filterProjects([project], "", true)).toHaveLength(0);
    expect(filterProjects([project], "python", false)).toHaveLength(0);
  });
  it("shows connection errors and retries the actual bridge", async () => {
    vi.mocked(dispatch)
      .mockRejectedValueOnce(new Error("Backend offline"))
      .mockResolvedValue({ roots: [], projects: [] });
    render(<App />);
    expect(await screen.findByRole("alert")).toBeTruthy();
    fireEvent.click(screen.getByText("Yeniden dene"));
    await waitFor(() => expect(screen.queryByRole("alert")).toBeNull());
  });
  it("persists a favorite using the selected project id", async () => {
    vi.mocked(dispatch).mockResolvedValue({ roots: [], projects: [project] });
    render(<App />);
    fireEvent.click(
      await screen.findByLabelText("Orbit Explorer favorilere ekle"),
    );
    await waitFor(() =>
      expect(dispatch).toHaveBeenCalledWith("favorite", {
        id: "p",
        value: true,
      }),
    );
  });
});
