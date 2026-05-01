import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./styles.css";

type DownloadFormat = "mp3" | "wav" | "m4a" | "flac" | "mp4" | "webm";

type Settings = {
  download_dir: string | null;
  format: DownloadFormat;
  audio_quality: string;
  ytdlp_channel: string;
};

type VideoEntry = {
  id: string;
  title: string;
  url: string;
  source?: string | null;
  thumbnail?: string | null;
  duration?: number | null;
  duration_string?: string | null;
  selected: boolean;
};

type PlaylistInfo = {
  title: string;
  entries: VideoEntry[];
  is_playlist: boolean;
};

type InitEvent = {
  kind: "starting" | "downloading" | "extracting" | "completed" | "failed";
  message: string;
  percent: number | null;
};

type DownloadEvent = {
  kind:
    | "starting"
    | "progress"
    | "message"
    | "converting"
    | "completed"
    | "failed"
    | "stopped"
    | "all_completed";
  current: number;
  total: number;
  percent: number;
  title: string;
  source?: string | null;
  message: string;
};

type SiteStatus = "stable" | "experimental" | "broken";

type SupportedSite = {
  name: string;
  status: SiteStatus;
};

type SupportedSites = {
  featured: SupportedSite[];
  extractors: SupportedSite[];
};

const formats: Array<{ value: DownloadFormat; label: string; tone: string }> = [
  { value: "mp3", label: "MP3", tone: "Audio" },
  { value: "m4a", label: "M4A", tone: "Audio" },
  { value: "flac", label: "FLAC", tone: "Lossless" },
  { value: "wav", label: "WAV", tone: "Lossless" },
  { value: "mp4", label: "MP4", tone: "Video" },
  { value: "webm", label: "WEBM", tone: "Video" },
];

const channels = ["stable", "nightly", "master"];

let settings: Settings = {
  download_dir: null,
  format: "mp3",
  audio_quality: "320K",
  ytdlp_channel: "stable",
};
let url = "";
let playlist: PlaylistInfo | null = null;
let phase:
  | "booting"
  | "idle"
  | "analyzing"
  | "ready"
  | "downloading"
  | "finished" = "booting";
let initMessage = "실행 환경을 준비하고 있습니다.";
let initPercent = 0;
let errorMessage = "";
let downloadMessage = "";
let downloadTitle = "";
let downloadPercent = 0;
let downloadCurrent = 0;
let downloadTotal = 0;
let downloadEventCount = 0;
let logs: string[] = [];
let supportPanelOpen = false;
let supportExpanded = false;
let supportQuery = "";
let supportLoading = false;
let supportError = "";
let supportedSites: SupportedSites | null = null;

const app = document.querySelector<HTMLDivElement>("#app")!;
const isTauriRuntime = "__TAURI_INTERNALS__" in window;

function formatDuration(entry: VideoEntry): string {
  if (entry.duration_string) return entry.duration_string;
  if (typeof entry.duration === "number") {
    const minutes = Math.floor(entry.duration / 60);
    const seconds = Math.floor(entry.duration % 60)
      .toString()
      .padStart(2, "0");
    return `${minutes}:${seconds}`;
  }
  return "--:--";
}

function selectedEntries(): VideoEntry[] {
  return playlist?.entries.filter((entry) => entry.selected) ?? [];
}

function isYoutubeUrl(value: string): boolean {
  try {
    const hostname = new URL(value).hostname.toLowerCase();
    return hostname === "youtu.be" || hostname.endsWith(".youtube.com") || hostname === "youtube.com";
  } catch {
    const normalized = value.toLowerCase();
    return normalized.includes("youtube.com/") || normalized.includes("youtu.be/");
  }
}

function sourceSupportCopy(value: string): string {
  if (!value.trim()) {
    return "YouTube is stable. Other yt-dlp compatible sites are experimental.";
  }
  return isYoutubeUrl(value)
    ? "Stable YouTube support"
    : "Experimental support through yt-dlp";
}

function sampleSupportedSites(): SupportedSites {
  return {
    featured: [
      { name: "YouTube", status: "stable" },
      { name: "YouTube Music", status: "stable" },
      { name: "TikTok", status: "experimental" },
      { name: "SoundCloud", status: "experimental" },
      { name: "Vimeo", status: "experimental" },
      { name: "X/Twitter", status: "experimental" },
      { name: "Instagram", status: "experimental" },
      { name: "Twitch", status: "experimental" },
      { name: "Facebook", status: "experimental" },
      { name: "Bilibili", status: "experimental" },
      { name: "Niconico", status: "experimental" },
    ],
    extractors: [
      { name: "youtube", status: "experimental" },
      { name: "TikTok", status: "experimental" },
      { name: "SoundCloud", status: "experimental" },
      { name: "Vimeo", status: "experimental" },
      { name: "247sports", status: "broken" },
    ],
  };
}

function statusLabel(status: SiteStatus): string {
  if (status === "stable") return "안정";
  if (status === "broken") return "불안정";
  return "실험";
}

function renderSitePill(site: SupportedSite): string {
  return `
    <span class="site-pill ${site.status}">
      <strong>${escapeHtml(site.name)}</strong>
      <em>${statusLabel(site.status)}</em>
    </span>
  `;
}

function filteredExtractors(): SupportedSite[] {
  const sites = supportedSites?.extractors ?? [];
  const query = supportQuery.trim().toLowerCase();
  if (!query) return sites;
  return sites.filter((site) => site.name.toLowerCase().includes(query));
}

function renderSourceBadge(source?: string | null): string {
  if (!source) return "";
  return `<span class="source-badge">${escapeHtml(source)}</span>`;
}

function renderExtractorList(): string {
  const extractors = filteredExtractors();
  return extractors.map(renderSitePill).join("") || `<p class="support-note">검색 결과가 없습니다.</p>`;
}

function withSource(value: string, source?: string | null): string {
  return source ? `${value} (${source})` : value;
}

async function toggleSupportPanel() {
  supportPanelOpen = !supportPanelOpen;
  if (!supportPanelOpen) {
    render();
    return;
  }

  if (supportedSites || supportLoading) {
    render();
    return;
  }

  supportLoading = true;
  supportError = "";
  render();

  if (!isTauriRuntime) {
    await new Promise((resolve) => window.setTimeout(resolve, 180));
    supportedSites = sampleSupportedSites();
    supportLoading = false;
    render();
    return;
  }

  try {
    supportedSites = await invoke<SupportedSites>("supported_sites");
  } catch (error) {
    supportError = String(error);
  } finally {
    supportLoading = false;
    render();
  }
}

function setLog(message: string) {
  if (!message.trim()) return;
  logs = [...logs.slice(-79), message];
}

async function persistSettings(partial: Partial<Settings>) {
  settings = { ...settings, ...partial };
  if (!isTauriRuntime) {
    render();
    return;
  }
  settings = await invoke<Settings>("save_settings", { settings });
  render();
}

async function chooseFolder() {
  if (!isTauriRuntime) {
    await persistSettings({ download_dir: "C:\\Users\\USER\\Downloads" });
    return;
  }
  const path = await invoke<string | null>("choose_folder");
  if (path) {
    await persistSettings({ download_dir: path });
  }
}

async function analyze() {
  if (!url.trim()) {
    errorMessage = "URL을 입력해 주세요.";
    render();
    return;
  }

  phase = "analyzing";
  errorMessage = "";
  playlist = null;
  render();

  if (!isTauriRuntime) {
    await new Promise((resolve) => window.setTimeout(resolve, 420));
    playlist = samplePlaylist();
    phase = "ready";
    render();
    return;
  }

  try {
    playlist = await invoke<PlaylistInfo>("analyze_url", {
      url: url.trim(),
      ytdlpChannel: settings.ytdlp_channel,
    });
    phase = "ready";
  } catch (error) {
    phase = "idle";
    errorMessage = String(error);
  }
  render();
}

async function startDownload() {
  if (!settings.download_dir) {
    errorMessage = "저장 폴더를 먼저 선택해 주세요.";
    render();
    return;
  }

  const entries = selectedEntries();
  if (entries.length === 0) {
    errorMessage = "다운로드할 영상을 선택해 주세요.";
    render();
    return;
  }

  phase = "downloading";
  errorMessage = "";
  logs = [];
  downloadPercent = 0;
  downloadCurrent = 1;
  downloadTotal = entries.length;
  downloadEventCount = 0;
  downloadMessage = "다운로드를 준비하고 있습니다.";
  render();

  if (!isTauriRuntime) {
    simulateDownload(entries);
    return;
  }

  try {
    await invoke("start_download", {
      entries,
      format: settings.format,
      outputDir: settings.download_dir,
    });
    window.setTimeout(() => {
      if (phase === "downloading" && downloadEventCount === 0) {
        downloadMessage = "다운로드 요청을 보냈지만 앱 응답이 아직 도착하지 않았습니다.";
        setLog(downloadMessage);
        render();
      }
    }, 2500);
  } catch (error) {
    phase = "ready";
    errorMessage = String(error);
    render();
  }
}

async function stopDownload() {
  if (!isTauriRuntime) {
    phase = "ready";
    downloadMessage = "전체 작업이 취소되었습니다.";
    setLog(downloadMessage);
    render();
    return;
  }
  downloadMessage = "전체 작업을 취소하는 중입니다.";
  setLog(downloadMessage);
  render();
  try {
    await invoke("stop_download");
    phase = "ready";
    downloadMessage = "전체 작업이 취소되었습니다.";
    setLog(downloadMessage);
    render();
  } catch (error) {
    phase = "ready";
    errorMessage = String(error);
    render();
  }
}

async function openFolder() {
  if (!isTauriRuntime) return;
  if (settings.download_dir) {
    await invoke("open_folder", { path: settings.download_dir });
  }
}

function toggleAll(selected: boolean) {
  if (!playlist) return;
  playlist.entries = playlist.entries.map((entry) => ({ ...entry, selected }));
  render();
}

function toggleEntry(id: string, selected: boolean) {
  if (!playlist) return;
  playlist.entries = playlist.entries.map((entry) =>
    entry.id === id ? { ...entry, selected } : entry,
  );
  render();
}

function render() {
  const selectedCount = selectedEntries().length;
  const totalCount = playlist?.entries.length ?? 0;
  const readyToDownload = phase === "ready" || phase === "finished";

  app.innerHTML = `
    <main class="shell">
      <aside class="sidebar">
        <div class="brand">
          <div class="brand-mark">SP</div>
          <div>
            <h1>Spull</h1>
          </div>
        </div>

        <section class="phase-strip" aria-label="작업 상태">
          ${renderPhase("booting", "준비")}
          ${renderPhase("idle", "분석")}
          ${renderPhase("ready", "선택")}
          ${renderPhase("downloading", "다운로드")}
        </section>

        <section class="panel input-panel">
          <div class="panel-head">
            <div>
              <p class="eyebrow">Source</p>
              <h3>Media URL</h3>
            </div>
            <button class="tiny-button" id="supportSitesButton">지원 사이트</button>
          </div>
          <div class="url-row">
            <input id="urlInput" value="${escapeHtml(url)}" placeholder="https://www.youtube.com/watch?v=... or any yt-dlp URL" />
            <button id="analyzeButton" ${phase === "analyzing" || phase === "downloading" ? "disabled" : ""}>${phase === "analyzing" ? "분석 중" : "분석"}</button>
          </div>
          <p class="support-note">${sourceSupportCopy(url)}</p>
          ${supportPanelOpen ? renderSupportPanel() : ""}
          <div class="path-row">
            <span>저장 위치</span>
            <strong title="${escapeHtml(settings.download_dir ?? "")}">${settings.download_dir ? escapeHtml(settings.download_dir) : "폴더를 선택해 주세요."}</strong>
            <button class="ghost-button" id="chooseFolder">폴더 선택</button>
          </div>
        </section>

        <section class="panel format-panel">
          <div class="panel-head">
            <div>
              <p class="eyebrow">Output</p>
              <h3>포맷</h3>
            </div>
            <span class="format-summary">${settings.format.toUpperCase()}</span>
          </div>
          <div class="format-grid">
            ${formats
              .map(
                (format) => `
                  <button class="format-chip ${settings.format === format.value ? "selected" : ""}" data-format="${format.value}">
                    <strong>${format.label}</strong>
                    <span>${format.tone}</span>
                  </button>
                `,
              )
              .join("")}
          </div>
        </section>

        <label class="channel-picker panel">
          <span>yt-dlp 채널</span>
          <select id="channelSelect">
            ${channels.map((channel) => `<option value="${channel}" ${channel === settings.ytdlp_channel ? "selected" : ""}>${channel}</option>`).join("")}
          </select>
        </label>
      </aside>

      <section class="workspace">
        ${errorMessage ? `<div class="error-banner">${escapeHtml(errorMessage)}</div>` : ""}

        ${phase === "booting" ? renderBootPanel() : renderQueuePanel(selectedCount, totalCount)}
        ${renderProgressPanel(readyToDownload, selectedCount)}
      </section>
    </main>
  `;

  bindEvents();
}

function renderPhase(target: typeof phase, label: string) {
  const active =
    target === phase ||
    (target === "idle" && phase === "analyzing") ||
    (target === "downloading" && phase === "finished");
  return `<span class="phase-tab ${active ? "active" : ""}">${label}</span>`;
}

function renderSupportPanel() {
  if (supportLoading) {
    return `
      <div class="support-panel">
        <div class="support-panel-head">
          <strong>지원 사이트</strong>
          <button class="tiny-button" id="supportClose">닫기</button>
        </div>
        <p class="support-note">yt-dlp 지원 목록을 불러오는 중입니다.</p>
      </div>
    `;
  }

  if (supportError) {
    return `
      <div class="support-panel">
        <div class="support-panel-head">
          <strong>지원 사이트</strong>
          <button class="tiny-button" id="supportClose">닫기</button>
        </div>
        <p class="support-error">${escapeHtml(supportError)}</p>
      </div>
    `;
  }

  const featured = supportedSites?.featured ?? [];
  return `
    <div class="support-panel">
      <div class="support-panel-head">
        <strong>지원 사이트</strong>
        <button class="tiny-button" id="supportClose">닫기</button>
      </div>
      <p class="support-note">대표 사이트를 먼저 보여주고, 전체 목록은 현재 설치된 yt-dlp 기준입니다.</p>
      <div class="site-pill-grid">
        ${featured.map(renderSitePill).join("")}
      </div>
      <button class="ghost-button support-toggle" id="supportExpand">
        ${supportExpanded ? "전체 목록 접기" : `전체 yt-dlp 목록 보기 (${supportedSites?.extractors.length ?? 0})`}
      </button>
      ${
        supportExpanded
          ? `
            <input id="supportSearch" class="support-search" value="${escapeHtml(supportQuery)}" placeholder="사이트 이름 검색" />
            <div class="extractor-list">
              ${renderExtractorList()}
            </div>
          `
          : ""
      }
    </div>
  `;
}

function renderBootPanel() {
  return `
    <section class="panel boot-panel">
      <div class="ring" style="--progress:${initPercent}"></div>
      <div>
        <p class="eyebrow">Initialize</p>
        <h3>실행 준비 중</h3>
        <p>${escapeHtml(initMessage)}</p>
      </div>
    </section>
  `;
}

function renderQueuePanel(selectedCount: number, totalCount: number) {
  if (!playlist) {
    return `
      <section class="panel queue-panel empty">
        <div class="empty-illustration"></div>
        <div>
          <p class="eyebrow">Ready</p>
          <h3>Paste a media URL to analyze it.</h3>
          <p>YouTube URLs are treated as stable. Other yt-dlp compatible sites are available as experimental sources.</p>
        </div>
      </section>
    `;
  }

  return `
    <section class="panel queue-panel">
      <div class="queue-head">
        <div>
          <p class="eyebrow">${playlist.is_playlist ? "Playlist" : "Single video"}</p>
          <h3>${escapeHtml(playlist.title)}</h3>
        </div>
        <div class="queue-actions">
          <span>${selectedCount}/${totalCount} 선택</span>
          <button class="tiny-button" id="selectAll">전체</button>
          <button class="tiny-button" id="clearAll">해제</button>
        </div>
      </div>
      <div class="video-list">
        ${playlist.entries
          .map(
            (entry, index) => `
              <article class="video-row">
                <label class="check-wrap">
                  <input type="checkbox" data-entry="${escapeHtml(entry.id)}" ${entry.selected ? "checked" : ""} />
                  <span></span>
                </label>
                <div class="thumb">${entry.thumbnail ? `<img src="${escapeHtml(entry.thumbnail)}" alt="" />` : `<span>${index + 1}</span>`}</div>
                <div class="video-meta">
                  <h4>${index + 1}. ${escapeHtml(entry.title)}</h4>
                  <div class="video-detail-row">
                    <span>${escapeHtml(formatDuration(entry))}</span>
                    ${renderSourceBadge(entry.source)}
                  </div>
                </div>
              </article>
            `,
          )
          .join("")}
      </div>
    </section>
  `;
}

function renderProgressPanel(readyToDownload: boolean, selectedCount: number) {
  const heading =
    phase === "downloading"
      ? downloadTitle || "다운로드 중"
      : phase === "finished"
        ? "작업이 완료되었습니다."
        : "다운로드 대기";
  const message =
    downloadMessage ||
    "다운로드를 시작하면 진행률과 로그가 이곳에 표시됩니다.";

  return `
    <section class="panel progress-panel">
      <div class="progress-head">
        <div>
          <p class="eyebrow">Progress</p>
          <h3>${escapeHtml(heading)}</h3>
        </div>
        <div class="actions">
          ${
            phase === "downloading"
              ? `<button class="danger-button" id="stopButton">전체 취소</button>`
              : `<button class="primary-button" id="downloadButton" ${!readyToDownload || selectedCount === 0 ? "disabled" : ""}>${selectedCount || 0}개 다운로드</button>`
          }
          <button class="ghost-button" id="openFolder" ${!settings.download_dir ? "disabled" : ""}>열기</button>
        </div>
      </div>
      <div class="progress-track"><span style="width:${Math.max(0, Math.min(downloadPercent, 100))}%"></span></div>
      <div class="progress-copy">
        <span>${phase === "downloading" ? `${downloadCurrent}/${downloadTotal}` : "대기"}</span>
        <strong>${escapeHtml(message)}</strong>
      </div>
      <div class="log-box">
        ${(logs.length ? logs : ["아직 로그가 없습니다."]).map((line) => `<p>${escapeHtml(line)}</p>`).join("")}
      </div>
    </section>
  `;
}

function bindEvents() {
  document
    .querySelector<HTMLButtonElement>("#chooseFolder")
    ?.addEventListener("click", chooseFolder);
  document
    .querySelector<HTMLButtonElement>("#analyzeButton")
    ?.addEventListener("click", analyze);
  document
    .querySelector<HTMLButtonElement>("#supportSitesButton")
    ?.addEventListener("click", toggleSupportPanel);
  document
    .querySelector<HTMLButtonElement>("#supportClose")
    ?.addEventListener("click", () => {
      supportPanelOpen = false;
      render();
    });
  document
    .querySelector<HTMLButtonElement>("#supportExpand")
    ?.addEventListener("click", () => {
      supportExpanded = !supportExpanded;
      render();
    });
  document
    .querySelector<HTMLInputElement>("#supportSearch")
    ?.addEventListener("input", (event) => {
      supportQuery = (event.target as HTMLInputElement).value;
      const list = document.querySelector<HTMLDivElement>(".extractor-list");
      if (list) list.innerHTML = renderExtractorList();
    });
  document
    .querySelector<HTMLInputElement>("#urlInput")
    ?.addEventListener("input", (event) => {
      url = (event.target as HTMLInputElement).value;
    });
  document
    .querySelector<HTMLInputElement>("#urlInput")
    ?.addEventListener("keydown", (event) => {
      if (event.key === "Enter") analyze();
    });
  document
    .querySelector<HTMLSelectElement>("#channelSelect")
    ?.addEventListener("change", (event) => {
      persistSettings({
        ytdlp_channel: (event.target as HTMLSelectElement).value,
      });
    });
  document
    .querySelectorAll<HTMLButtonElement>("[data-format]")
    .forEach((button) => {
      button.addEventListener("click", () =>
        persistSettings({ format: button.dataset.format as DownloadFormat }),
      );
    });
  document
    .querySelector<HTMLButtonElement>("#selectAll")
    ?.addEventListener("click", () => toggleAll(true));
  document
    .querySelector<HTMLButtonElement>("#clearAll")
    ?.addEventListener("click", () => toggleAll(false));
  document
    .querySelectorAll<HTMLInputElement>("[data-entry]")
    .forEach((checkbox) => {
      checkbox.addEventListener("change", () =>
        toggleEntry(checkbox.dataset.entry!, checkbox.checked),
      );
    });
  document
    .querySelector<HTMLButtonElement>("#downloadButton")
    ?.addEventListener("click", startDownload);
  document
    .querySelector<HTMLButtonElement>("#stopButton")
    ?.addEventListener("click", stopDownload);
  document
    .querySelector<HTMLButtonElement>("#openFolder")
    ?.addEventListener("click", openFolder);
}

function escapeHtml(value: string) {
  return value.replace(/[&<>"']/g, (char) => {
    const entities: Record<string, string> = {
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;",
      '"': "&quot;",
      "'": "&#39;",
    };
    return entities[char];
  });
}

async function boot() {
  if (!isTauriRuntime) {
    phase = "idle";
    render();
    return;
  }

  await listen<InitEvent>("init-progress", (event) => {
    const payload = event.payload;
    initMessage = payload.message;
    initPercent = payload.percent ?? initPercent;
    if (payload.kind === "completed") {
      phase = "idle";
      initPercent = 100;
    }
    if (payload.kind === "failed") {
      phase = "idle";
      errorMessage = payload.message;
    }
    render();
  });

  await listen<DownloadEvent>("download-progress", (event) => {
    const payload = event.payload;
    downloadEventCount += 1;
    downloadCurrent = payload.current;
    downloadTotal = payload.total;
    downloadPercent = payload.percent;
    downloadTitle = withSource(payload.title, payload.source);
    downloadMessage = payload.message;

    if (payload.kind === "starting" || payload.kind === "message" || payload.kind === "converting") {
      setLog(withSource(payload.message, payload.source));
    }
    if (payload.kind === "completed") setLog(`완료: ${withSource(payload.title, payload.source)}`);
    if (payload.kind === "failed") {
      phase = "ready";
      errorMessage = payload.message;
      setLog(payload.message);
    }
    if (payload.kind === "stopped") {
      phase = "ready";
      downloadMessage = payload.message || "전체 작업이 취소되었습니다.";
      setLog(downloadMessage);
    }
    if (payload.kind === "all_completed") {
      phase = "finished";
      downloadPercent = 100;
      setLog("모든 작업이 완료되었습니다.");
    }
    render();
  });

  settings = await invoke<Settings>("get_settings");
  render();
  await invoke("initialize", { ytdlpChannel: settings.ytdlp_channel });
}

function samplePlaylist(): PlaylistInfo {
  return {
    title: "Spull preview queue",
    is_playlist: true,
    entries: [
      {
        id: "preview-1",
        title: "Stable YouTube sample",
        url: "https://youtu.be/preview-1",
        source: "YouTube",
        thumbnail: null,
        duration: 245,
        duration_string: "4:05",
        selected: true,
      },
      {
        id: "preview-2",
        title: "Experimental short-form clip sample",
        url: "https://www.tiktok.com/@example/video/preview-2",
        source: "TikTok",
        thumbnail: null,
        duration: 632,
        duration_string: "10:32",
        selected: true,
      },
      {
        id: "preview-3",
        title: "Experimental audio source sample",
        url: "https://soundcloud.com/example/preview-3",
        source: "SoundCloud",
        thumbnail: null,
        duration: 188,
        duration_string: "3:08",
        selected: false,
      },
    ],
  };
}

function simulateDownload(entries: VideoEntry[]) {
  let tick = 0;
  const totalTicks = 30;
  const timer = window.setInterval(() => {
    tick += 1;
    downloadCurrent = Math.min(
      entries.length,
      Math.floor((tick / totalTicks) * entries.length) + 1,
    );
    downloadTotal = entries.length;
    downloadPercent = Math.min(100, Math.round((tick / totalTicks) * 100));
    const currentEntry = entries[Math.min(entries.length - 1, downloadCurrent - 1)];
    downloadTitle = currentEntry
      ? withSource(currentEntry.title, currentEntry.source)
      : "다운로드 중";
    downloadMessage = `${downloadPercent}%`;
    if (tick % 6 === 0)
      setLog(`[preview] ${downloadMessage} - ${downloadTitle}`);
    if (tick >= totalTicks) {
      window.clearInterval(timer);
      phase = "finished";
      downloadPercent = 100;
      downloadMessage = "모든 작업이 완료되었습니다.";
      setLog(downloadMessage);
    }
    render();
  }, 120);
}

render();
boot().catch((error) => {
  phase = "idle";
  errorMessage = String(error);
  render();
});
