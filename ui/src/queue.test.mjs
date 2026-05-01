import assert from "node:assert/strict";
import test from "node:test";
import {
  ensureUrlRow,
  mergePlaylistResults,
  nonEmptyUrlRows,
} from "../../.test-build/queue.js";

test("nonEmptyUrlRows trims rows and ignores empty values", () => {
  const rows = [
    { id: "url-1", value: "  https://youtu.be/one  " },
    { id: "url-2", value: " " },
    { id: "url-3", value: "https://example.com/two" },
  ];

  assert.deepEqual(nonEmptyUrlRows(rows), [
    { id: "url-1", value: "https://youtu.be/one" },
    { id: "url-3", value: "https://example.com/two" },
  ]);
});

test("ensureUrlRow keeps at least one input row", () => {
  assert.deepEqual(ensureUrlRow([], () => "url-new"), [
    { id: "url-new", value: "" },
  ]);
});

test("mergePlaylistResults combines entries and gives them unique ids", () => {
  const first = {
    title: "First",
    is_playlist: false,
    entries: [
      {
        id: "same-id",
        title: "First video",
        url: "https://youtu.be/first",
        selected: true,
      },
    ],
  };
  const second = {
    title: "Second list",
    is_playlist: true,
    entries: [
      {
        id: "same-id",
        title: "Second video",
        url: "https://youtu.be/second",
        selected: true,
      },
    ],
  };

  const merged = mergePlaylistResults([first, second]);

  assert.equal(merged.title, "2 links");
  assert.equal(merged.is_playlist, true);
  assert.deepEqual(
    merged.entries.map((entry) => entry.id),
    ["source-1-same-id", "source-2-same-id"],
  );
  assert.deepEqual(
    merged.entries.map((entry) => entry.title),
    ["First video", "Second video"],
  );
});
