export type DownloadFormat = "mp3" | "wav" | "m4a" | "flac" | "mp4" | "webm";

export type UrlRow = {
  id: string;
  value: string;
};

export type VideoEntry = {
  id: string;
  title: string;
  url: string;
  source?: string | null;
  thumbnail?: string | null;
  duration?: number | null;
  duration_string?: string | null;
  selected: boolean;
};

export type PlaylistInfo = {
  title: string;
  entries: VideoEntry[];
  is_playlist: boolean;
};

export function nonEmptyUrlRows(rows: UrlRow[]): UrlRow[] {
  return rows
    .map((row) => ({ ...row, value: row.value.trim() }))
    .filter((row) => row.value.length > 0);
}

export function ensureUrlRow(rows: UrlRow[], nextId: () => string): UrlRow[] {
  return rows.length > 0 ? rows : [{ id: nextId(), value: "" }];
}

export function mergePlaylistResults(results: PlaylistInfo[]): PlaylistInfo {
  const entries = results.flatMap((result, resultIndex) =>
    result.entries.map((entry, entryIndex) => ({
      ...entry,
      id: uniqueEntryId(entry, resultIndex, entryIndex),
      selected: entry.selected ?? true,
    })),
  );

  return {
    title: results.length === 1 ? results[0].title : `${results.length} links`,
    entries,
    is_playlist: results.length > 1 || results.some((result) => result.is_playlist),
  };
}

function uniqueEntryId(
  entry: VideoEntry,
  resultIndex: number,
  entryIndex: number,
): string {
  const rawId = entry.id || entry.url || `${entryIndex + 1}`;
  return `source-${resultIndex + 1}-${rawId}`;
}
