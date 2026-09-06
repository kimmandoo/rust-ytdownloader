import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:file_selector/file_selector.dart';

import '../models/media_models.dart';
import '../services/spull_backend.dart';

enum AppPhase { booting, idle, analyzing, ready, downloading, finished }

class SpullController extends ChangeNotifier {
  SpullController({SpullBackend? backend})
    : _backend = backend ?? SpullBackend();

  final SpullBackend _backend;
  final List<UrlRow> urlRows = <UrlRow>[UrlRow(id: 'url-1')];
  final List<String> logs = <String>[];

  AppSettings settings = const AppSettings();
  PlaylistInfo? playlist;
  SupportedSites? sites;
  AppPhase phase = AppPhase.booting;
  String initMessage = '앱을 준비하는 중...';
  double initPercent = 0;
  bool initIndeterminate = false;
  String errorMessage = '';
  String downloadMessage = '다운로드를 시작하면 진행률이 표시됩니다.';
  String downloadTitle = '';
  double downloadPercent = 0;
  int downloadCurrent = 0;
  int downloadTotal = 0;
  bool supportPanelOpen = false;
  bool supportLoading = false;
  String supportError = '';
  String supportQuery = '';

  StreamSubscription<DownloadEvent>? _downloadSubscription;
  Timer? _downloadClock;
  DateTime? _downloadStartedAt;
  Duration downloadElapsed = Duration.zero;
  int _analysisRequestId = 0;
  int _supportRequestId = 0;
  int _nextUrlId = 2;

  bool get hasDownloadableSelection => selectedEntries.isNotEmpty;
  List<VideoEntry> get selectedEntries =>
      playlist?.entries.where((entry) => entry.selected).toList() ??
      <VideoEntry>[];
  int get selectedCount => selectedEntries.length;
  int get totalCount => playlist?.entries.length ?? 0;
  bool get readyToDownload =>
      phase == AppPhase.ready || phase == AppPhase.finished;
  bool get isBusy =>
      phase == AppPhase.analyzing || phase == AppPhase.downloading;
  bool get isReadyForInput => phase != AppPhase.booting;
  String get elapsedLabel => _clockLabel(downloadElapsed);
  String get etaLabel {
    if (downloadPercent <= 0 || downloadElapsed.inSeconds < 2) return '--:--';
    final totalSeconds = (downloadElapsed.inSeconds * 100 / downloadPercent)
        .round();
    return _clockLabel(
      Duration(
        seconds: (totalSeconds - downloadElapsed.inSeconds)
            .clamp(0, 864000)
            .toInt(),
      ),
    );
  }

  Future<void> boot() async {
    settings = await _backend.loadSettings();
    notifyListeners();
    await for (final event in _backend.initialize(settings.ytdlpChannel)) {
      initMessage = event.message;
      initIndeterminate = event.percent == null;
      initPercent = event.percent ?? initPercent;
      if (event.kind == InitKind.ready || event.kind == InitKind.failed) {
        phase = AppPhase.idle;
        if (event.kind == InitKind.failed) errorMessage = event.message;
      }
      notifyListeners();
    }
    if (phase == AppPhase.booting) phase = AppPhase.idle;
    notifyListeners();
  }

  Future<void> setFormat(DownloadFormat format) =>
      _saveSettings(settings.copyWith(format: format));

  Future<void> setAudioQuality(String quality) =>
      _saveSettings(settings.copyWith(audioQuality: quality));

  Future<void> setVideoQuality(String quality) =>
      _saveSettings(settings.copyWith(videoQuality: quality));

  Future<void> setChannel(String channel) =>
      _saveSettings(settings.copyWith(ytdlpChannel: channel));

  Future<void> setCookieBrowser(String browser) =>
      _saveSettings(settings.copyWith(cookieBrowser: browser));

  Future<void> chooseFolder() async {
    final initialDirectory = settings.downloadDir?.isNotEmpty == true
        ? settings.downloadDir
        : _backend.defaultDownloadDirectory;
    final path = await getDirectoryPath(
      initialDirectory: initialDirectory,
      confirmButtonText: '이 폴더 사용',
      canCreateDirectories: true,
    );
    if (path != null && path.isNotEmpty) {
      await _saveSettings(settings.copyWith(downloadDir: path));
    }
  }

  Future<void> chooseCookieFile() async {
    const typeGroup = XTypeGroup(label: 'Cookies', extensions: <String>['txt']);
    final file = await openFile(acceptedTypeGroups: <XTypeGroup>[typeGroup]);
    if (file != null) {
      await _saveSettings(settings.copyWith(cookieFile: file.path));
    }
  }

  Future<void> clearCookieFile() =>
      _saveSettings(settings.copyWith(clearCookieFile: true));
  void clearError() {
    if (errorMessage.isEmpty) return;
    errorMessage = '';
    notifyListeners();
  }

  void addUrlRow() {
    urlRows.add(UrlRow(id: 'url-${_nextUrlId++}'));
    notifyListeners();
  }

  void removeUrlRow(String id) {
    urlRows.removeWhere((row) => row.id == id);
    if (urlRows.isEmpty) urlRows.add(UrlRow(id: 'url-${_nextUrlId++}'));
    notifyListeners();
  }

  void updateUrl(String id, String value) {
    for (final row in urlRows) {
      if (row.id == id) row.value = value;
    }
  }

  void toggleAll(bool selected) {
    for (final entry in playlist?.entries ?? <VideoEntry>[]) {
      entry.selected = selected;
    }
    notifyListeners();
  }

  void toggleEntry(String id, bool selected) {
    for (final entry in playlist?.entries ?? <VideoEntry>[]) {
      if (entry.id == id) entry.selected = selected;
    }
    notifyListeners();
  }

  Future<void> analyze() async {
    final values = urlRows
        .map((row) => row.value.trim())
        .where((value) => value.isNotEmpty)
        .toList();
    if (values.isEmpty) {
      errorMessage = '분석할 URL을 입력해 주세요.';
      notifyListeners();
      return;
    }

    final requestId = ++_analysisRequestId;
    phase = AppPhase.analyzing;
    playlist = null;
    errorMessage = '';
    logs.clear();
    notifyListeners();

    final results = <PlaylistInfo>[];
    final failures = <String>[];
    for (var index = 0; index < values.length; index += 1) {
      if (requestId != _analysisRequestId) return;
      final value = values[index];
      final label = '${index + 1}/${values.length}';
      _log('[$label] 링크 분석 중: $value');
      notifyListeners();
      try {
        final info = await _backend.analyzeUrl(url: value, settings: settings);
        if (requestId != _analysisRequestId) return;
        results.add(info);
        _log('[$label] 분석 완료: ${info.title}');
      } on SpullOperationCancelled {
        if (requestId == _analysisRequestId) {
          phase = AppPhase.idle;
          _log('분석을 취소했습니다.');
          notifyListeners();
        }
        return;
      } catch (error) {
        if (requestId != _analysisRequestId) return;
        final message = '[$label] 분석 실패: $error';
        failures.add(message);
        _log(message);
      }
      notifyListeners();
    }

    if (requestId != _analysisRequestId) return;
    if (results.isEmpty) {
      phase = AppPhase.idle;
      errorMessage = failures.join('\n').isEmpty
          ? '분석에 실패했습니다.'
          : failures.join('\n');
      notifyListeners();
      return;
    }
    final entries = <VideoEntry>[];
    for (var sourceIndex = 0; sourceIndex < results.length; sourceIndex += 1) {
      for (
        var entryIndex = 0;
        entryIndex < results[sourceIndex].entries.length;
        entryIndex += 1
      ) {
        final entry = results[sourceIndex].entries[entryIndex];
        entries.add(
          VideoEntry(
            id: 'source-${sourceIndex + 1}-${entryIndex + 1}-${entry.id}',
            title: entry.title,
            url: entry.url,
            source: entry.source,
            thumbnail: entry.thumbnail,
            duration: entry.duration,
            durationString: entry.durationString,
            selected: entry.selected,
          ),
        );
      }
    }
    playlist = PlaylistInfo(
      title: results.length == 1
          ? results.first.title
          : '${results.length}개 링크',
      entries: entries,
      isPlaylist:
          results.length > 1 || results.any((result) => result.isPlaylist),
    );
    phase = AppPhase.ready;
    if (failures.isNotEmpty) {
      errorMessage = '${failures.length}개 링크는 분석하지 못했습니다.';
    }
    notifyListeners();
  }

  Future<void> cancelAnalyze() async {
    if (phase != AppPhase.analyzing) return;
    _analysisRequestId++;
    try {
      await _backend.cancelAnalysis();
    } finally {
      if (phase == AppPhase.analyzing) {
        phase = AppPhase.idle;
        _log('분석을 취소했습니다.');
        notifyListeners();
      }
    }
  }

  Future<void> startDownload() async {
    if (settings.downloadDir == null || settings.downloadDir!.isEmpty) {
      errorMessage = '저장 폴더를 먼저 선택해 주세요.';
      notifyListeners();
      return;
    }
    if (selectedEntries.isEmpty) {
      errorMessage = '다운로드할 항목을 선택해 주세요.';
      notifyListeners();
      return;
    }

    await _downloadSubscription?.cancel();
    phase = AppPhase.downloading;
    errorMessage = '';
    logs.clear();
    downloadPercent = 0;
    downloadCurrent = 1;
    downloadTotal = selectedEntries.length;
    downloadMessage = '다운로드를 준비하고 있습니다.';
    downloadTitle = selectedEntries.first.title;
    _startDownloadClock();
    notifyListeners();

    final stream = _backend.startDownload(
      entries: selectedEntries,
      settings: settings,
    );
    _downloadSubscription = stream.listen(
      _handleDownloadEvent,
      onError: (Object error) {
        _stopDownloadClock();
        phase = AppPhase.ready;
        errorMessage = '$error';
        notifyListeners();
      },
    );
  }

  Future<void> stopDownload() async {
    downloadMessage = '전체 다운로드를 취소하는 중...';
    _log(downloadMessage);
    notifyListeners();
    await _backend.stopDownload();
    _stopDownloadClock();
    phase = AppPhase.ready;
    downloadMessage = '전체 작업을 취소했습니다.';
    notifyListeners();
  }

  Future<void> openFolder() async {
    final path = settings.downloadDir;
    if (path == null || path.isEmpty) return;
    try {
      await _backend.openFolder(path);
    } catch (error) {
      errorMessage = '$error';
      notifyListeners();
    }
  }

  Future<void> toggleSupportPanel() async {
    supportPanelOpen = !supportPanelOpen;
    notifyListeners();
    if (!supportPanelOpen || sites != null || supportLoading) return;
    await _loadSupportedSites();
  }

  Future<void> retrySupportedSites() async {
    if (!supportPanelOpen || supportLoading) return;
    sites = null;
    await _loadSupportedSites();
  }

  Future<void> _loadSupportedSites() async {
    final requestId = ++_supportRequestId;
    supportLoading = true;
    supportError = '';
    notifyListeners();
    try {
      final loaded = await _backend.supportedSites();
      if (requestId != _supportRequestId) return;
      sites = loaded;
    } catch (error) {
      if (requestId != _supportRequestId) return;
      supportError = '$error';
    } finally {
      if (requestId == _supportRequestId) {
        supportLoading = false;
        notifyListeners();
      }
    }
  }

  Future<void> cancelSupportedSites() async {
    if (!supportLoading) return;
    _supportRequestId++;
    try {
      await _backend.cancelSupportedSites();
    } finally {
      supportLoading = false;
      supportError = 'extractor 목록 로딩을 취소했습니다.';
      notifyListeners();
    }
  }

  List<SupportedSite> get filteredExtractors {
    final query = supportQuery.trim().toLowerCase();
    final extractors = sites?.extractors ?? <SupportedSite>[];
    if (query.isEmpty) return extractors;
    return extractors
        .where((site) => site.name.toLowerCase().contains(query))
        .toList();
  }

  void updateSupportQuery(String query) {
    supportQuery = query;
    notifyListeners();
  }

  void _handleDownloadEvent(DownloadEvent event) {
    downloadCurrent = event.current;
    downloadTotal = event.total;
    downloadPercent = event.percent;
    downloadTitle = event.source == null
        ? event.title
        : '${event.title} · ${event.source}';
    downloadMessage = event.message;
    if (event.kind == 'starting' ||
        event.kind == 'failed' ||
        event.kind == 'stalled') {
      _log(event.message);
    }
    if (event.kind == 'completed') _log('다운로드 완료: $downloadTitle');
    if (event.kind == 'failed') {
      _stopDownloadClock();
      phase = AppPhase.ready;
      errorMessage = event.message;
    } else if (event.kind == 'stopped') {
      _stopDownloadClock();
      phase = AppPhase.ready;
    } else if (event.kind == 'all_completed') {
      _stopDownloadClock();
      phase = AppPhase.finished;
      downloadPercent = 100;
      downloadMessage = '모든 미디어를 저장했습니다.';
    }
    notifyListeners();
  }

  void _startDownloadClock() {
    _downloadClock?.cancel();
    _downloadStartedAt = DateTime.now();
    downloadElapsed = Duration.zero;
    _downloadClock = Timer.periodic(const Duration(seconds: 1), (_) {
      final startedAt = _downloadStartedAt;
      if (startedAt == null) return;
      downloadElapsed = DateTime.now().difference(startedAt);
      notifyListeners();
    });
  }

  void _stopDownloadClock() {
    _downloadClock?.cancel();
    _downloadClock = null;
    final startedAt = _downloadStartedAt;
    if (startedAt != null) {
      downloadElapsed = DateTime.now().difference(startedAt);
    }
  }

  String _clockLabel(Duration duration) {
    final hours = duration.inHours;
    final minutes = duration.inMinutes.remainder(60).toString().padLeft(2, '0');
    final seconds = duration.inSeconds.remainder(60).toString().padLeft(2, '0');
    return hours > 0 ? '$hours:$minutes:$seconds' : '$minutes:$seconds';
  }

  Future<void> _saveSettings(AppSettings next) async {
    settings = await _backend.saveSettings(next);
    notifyListeners();
  }

  void _log(String message) {
    if (message.trim().isEmpty) return;
    logs.add(message);
    if (logs.length > 80) logs.removeAt(0);
  }

  @override
  void dispose() {
    _downloadSubscription?.cancel();
    _stopDownloadClock();
    unawaited(_backend.stopDownload());
    unawaited(_backend.cancelAnalysis());
    unawaited(_backend.cancelSupportedSites());
    super.dispose();
  }
}
