export type Root = { id: string; path: string; scannedAt: number };
export type Project = {
  id: string;
  rootId: string;
  name: string;
  path: string;
  stack: string[];
  scripts: string[];
  favorite: boolean;
};
export type Snapshot = { roots: Root[]; projects: Project[] };
export type GitInfo = {
  available: boolean;
  branch: string;
  changes: { status: string; path: string }[];
  commits: { hash: string; subject: string; time: number }[];
  error: string | null;
};
