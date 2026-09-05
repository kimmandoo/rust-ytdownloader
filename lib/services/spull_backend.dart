import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:archive/archive.dart';
import 'package:path/path.dart' as p;

import '../models/media_models.dart';

/// Native desktop backend implemented in Dart.
///
/// It keeps the UI platform-neutral while delegating media work to the
/// user-installed or app-local yt-dlp and ffmpeg executables.
class SpullBackend {
  Process? _downloadProcess;
  bool _stopRequested = false;

  Directory get dataDirectory {
    final environment = Platform.environment;
    final root = Platform.isWindows
        ? (environment['LOCALAPPDATA'] ??
              environment['APPDATA'] ??
              Directory.systemTemp.path)
        : Platform.isMacOS
        ? p.join(
            environment['HOME'] ?? Directory.systemTemp.path,
            'Library',
            'Application Support',
          )
        : p.join(environment['HOME'] ?? Directory.systemTemp.path, '.config');
    return Directory(p.join(root, 'Spull'));
  }

  String get defaultDownloadDirectory {
    final environment = Platform.environment;
    final home = Platform.isWindows
        ? (environment['USERPROFILE'] ??
              environment['HOME'] ??
              Directory.systemTemp.path)
        : (environment['HOME'] ?? Directory.systemTemp.path);
    return p.join(home, 'Downloads');
  }

  Directory get binDirectory => Directory(p.join(dataDirectory.path, 'bin'));

  File get settingsFile => File(p.join(dataDirectory.path, 'settings.json'));

  Future<AppSettings> loadSettings() async {
    AppSettings loaded = const AppSettings();
    try {
      if (await settingsFile.exists()) {
        final content = await settingsFile.readAsString();
        final json = jsonDecode(content);
        if (json is Map<String, dynamic>) {
          loaded = AppSettings.fromJson(json);
        }
      }
    } catch (_) {
      // A damaged config must not prevent the app from opening.
    }

    if (loaded.downloadDir != null && loaded.downloadDir!.isNotEmpty) {
      return loaded;
    }

    final defaultDirectory = Directory(defaultDownloadDirectory);
    try {
      await defaultDirectory.create(recursive: true);
      final withDefault = loaded.copyWith(downloadDir: defaultDirectory.path);
      await saveSettings(withDefault);
      return withDefault;
    } catch (_) {
      return loaded;
    }
  }

  Future<AppSettings> saveSettings(AppSettings settings) async {
    await dataDirectory.create(recursive: true);
    await settingsFile.writeAsString('${settings.encode()}\n');
    return settings;
  }

  Stream<InitEvent> initialize(String channel) {
    final events = StreamController<InitEvent>();
    unawaited(_initialize(channel, events));
    return events.stream;
  }

  Future<void> _initialize(
    String channel,
    StreamController<InitEvent> events,
  ) async {
    try {
      await dataDirectory.create(recursive: true);
      await binDirectory.create(recursive: true);
      events.add(
        const InitEvent(
          kind: InitKind.starting,
          message: '앱을 준비하는 중...',
          percent: 5,
        ),
      );

      var lastProgress = <String, double>{};
      void reportDownload(
        String tool,
        double base,
        double span,
        double progress,
      ) {
        final rounded = (progress * 100).floorToDouble();
        if (lastProgress[tool] == rounded && progress < 1) return;
        lastProgress[tool] = rounded;
        events.add(
          InitEvent(
            kind: InitKind.checking,
            message: '$tool 자동 설치 중... ${rounded.round()}%',
            percent: base + span * progress,
          ),
        );
      }

      var ytdlpReady = await _commandWorks(ytdlpExecutable, const [
        '--version',
      ]);
      if (!ytdlpReady) {
        try {
          await _installYtdlp(
            (progress) => reportDownload('yt-dlp', 12, 25, progress),
          );
          ytdlpReady = await _commandWorks(ytdlpExecutable, const [
            '--version',
          ]);
        } catch (error) {
          events.add(
            InitEvent(
              kind: InitKind.failed,
              message: 'yt-dlp 자동 설치 실패: $error',
              percent: 0,
            ),
          );
          return;
        }
      }
      if (!ytdlpReady) {
        events.add(
          const InitEvent(
            kind: InitKind.failed,
            message: 'yt-dlp를 준비하지 못했습니다.',
            percent: 0,
          ),
        );
        return;
      }
      events.add(
        InitEvent(
          kind: InitKind.checking,
          message: 'yt-dlp ${channel.toUpperCase()} 채널 연결 완료',
          percent: 42,
        ),
      );

      var ffmpegReady = await _commandWorks(ffmpegExecutable, const [
        '-version',
      ]);
      if (!ffmpegReady) {
        try {
          await _installFfmpeg(
            (progress) => reportDownload('ffmpeg', 48, 24, progress),
          );
          ffmpegReady = await _commandWorks(ffmpegExecutable, const [
            '-version',
          ]);
        } catch (error) {
          events.add(
            InitEvent(
              kind: InitKind.failed,
              message: 'ffmpeg 자동 설치 실패: $error',
              percent: 0,
            ),
          );
          return;
        }
      }
      if (!ffmpegReady) {
        events.add(
          const InitEvent(
            kind: InitKind.failed,
            message: 'ffmpeg를 준비하지 못했습니다.',
            percent: 0,
          ),
        );
        return;
      }

      var denoReady = await _commandWorks(denoExecutable, const ['--version']);
      if (!denoReady) {
        try {
          await _installDeno(
            (progress) => reportDownload('Deno', 76, 18, progress),
          );
          denoReady = await _commandWorks(denoExecutable, const ['--version']);
        } catch (_) {
          // Deno improves extractor compatibility but is optional.
        }
      }

      final runtime = denoReady ? ' · Deno 준비 완료' : ' · Deno 선택 설치 생략';
      events.add(
        InitEvent(
          kind: InitKind.checking,
          message: 'ffmpeg 준비 완료$runtime',
          percent: 96,
        ),
      );
      await Future<void>.delayed(const Duration(milliseconds: 180));
      events.add(
        const InitEvent(kind: InitKind.ready, message: '준비 완료', percent: 100),
      );
    } catch (error) {
      events.add(
        InitEvent(
          kind: InitKind.failed,
          message: '실행 환경 초기화 실패: $error',
          percent: 0,
        ),
      );
    } finally {
      await events.close();
    }
  }

  Future<void> _installYtdlp(void Function(double) onProgress) async {
    final destination = File(
      p.join(binDirectory.path, Platform.isWindows ? 'yt-dlp.exe' : 'yt-dlp'),
    );
    final partial = File('${destination.path}.part');
    await _downloadAsset(_ytdlpUrl, partial, onProgress);
    await _replaceFile(partial, destination);
    await _markExecutable(destination);
  }

  Future<void> _installFfmpeg(void Function(double) onProgress) async {
    final linux = Platform.isLinux;
    final archiveFile = File(
      p.join(
        binDirectory.path,
        linux ? '.ffmpeg-download.tar.xz' : '.ffmpeg-download.zip',
      ),
    );
    try {
      late final String url;
      if (Platform.isWindows) {
        url = _windowsFfmpegUrl;
      } else if (Platform.isMacOS) {
        url = _macFfmpegUrl;
      } else if (linux) {
        url = await _linuxFfmpegUrl();
      } else {
        throw '이 운영체제의 자동 ffmpeg 설치 주소가 없습니다.';
      }
      await _downloadAsset(url, archiveFile, onProgress);
      if (linux) {
        await _extractTarXzBinary(archiveFile, 'ffmpeg');
      } else {
        await _extractZipBinary(
          archiveFile,
          Platform.isWindows ? 'ffmpeg.exe' : 'ffmpeg',
        );
      }
    } finally {
      if (await archiveFile.exists()) await archiveFile.delete();
    }
  }

  Future<void> _installDeno(void Function(double) onProgress) async {
    final archiveFile = File(p.join(binDirectory.path, '.deno-download.zip'));
    try {
      await _downloadAsset(await _denoUrl(), archiveFile, onProgress);
      await _extractZipBinary(
        archiveFile,
        Platform.isWindows ? 'deno.exe' : 'deno',
      );
    } finally {
      if (await archiveFile.exists()) await archiveFile.delete();
    }
  }

  Future<void> _downloadAsset(
    String url,
    File destination,
    void Function(double) onProgress,
  ) async {
    final client = HttpClient()
      ..userAgent = 'Spull/1.2 (desktop media downloader)'
      ..connectionTimeout = const Duration(seconds: 20)
      ..idleTimeout = const Duration(seconds: 30);
    IOSink? sink;
    try {
      final request = await client.getUrl(Uri.parse(url));
      request.followRedirects = true;
      final response = await request.close();
      if (response.statusCode < 200 || response.statusCode >= 300) {
        throw 'HTTP ${response.statusCode}';
      }
      await destination.parent.create(recursive: true);
      sink = destination.openWrite();
      var received = 0;
      await for (final chunk in response) {
        sink.add(chunk);
        received += chunk.length;
        if (response.contentLength > 0) {
          onProgress(received / response.contentLength);
        }
      }
      await sink.flush();
    } finally {
      await sink?.close();
      client.close(force: true);
    }
  }

  Future<void> _extractZipBinary(File archiveFile, String targetName) async {
    final archive = ZipDecoder().decodeBytes(await archiveFile.readAsBytes());
    ArchiveFile? match;
    for (final entry in archive.files) {
      if (entry.isFile &&
          p.basename(entry.name.replaceAll('\\', '/')) == targetName) {
        match = entry;
        break;
      }
    }
    if (match == null) throw '$targetName 파일이 압축 파일에 없습니다.';
    final bytes = match.readBytes();
    if (bytes == null || bytes.isEmpty) throw '$targetName 파일이 비어 있습니다.';
    final destination = File(p.join(binDirectory.path, targetName));
    await destination.writeAsBytes(bytes, flush: true);
    await _markExecutable(destination);
  }

  Future<void> _extractTarXzBinary(File archiveFile, String targetName) async {
    final tarBytes = XZDecoder().decodeBytes(await archiveFile.readAsBytes());
    final archive = TarDecoder().decodeBytes(tarBytes);
    ArchiveFile? match;
    for (final entry in archive.files) {
      if (entry.isFile &&
          p.basename(entry.name.replaceAll('\\', '/')) == targetName) {
        match = entry;
        break;
      }
    }
    if (match == null) throw '$targetName 파일이 압축 파일에 없습니다.';
    final bytes = match.readBytes();
    if (bytes == null || bytes.isEmpty) throw '$targetName 파일이 비어 있습니다.';
    final destination = File(p.join(binDirectory.path, targetName));
    await destination.writeAsBytes(bytes, flush: true);
    await _markExecutable(destination);
  }

  Future<void> _replaceFile(File partial, File destination) async {
    if (await destination.exists()) await destination.delete();
    await partial.rename(destination.path);
  }

  Future<void> _markExecutable(File file) async {
    if (Platform.isWindows) return;
    final result = await Process.run('chmod', <String>['+x', file.path]);
    if (result.exitCode != 0) throw '실행 권한 설정 실패: ${result.stderr}';
  }

  Future<String> _denoUrl() async {
    final architecture = await _machineArchitecture();
    if (Platform.isWindows) {
      return 'https://github.com/denoland/deno/releases/latest/download/deno-${architecture == 'arm64' ? 'aarch64' : 'x86_64'}-pc-windows-msvc.zip';
    }
    if (Platform.isMacOS) {
      return 'https://github.com/denoland/deno/releases/latest/download/deno-${architecture == 'arm64' ? 'aarch64' : 'x86_64'}-apple-darwin.zip';
    }
    if (Platform.isLinux) {
      return 'https://github.com/denoland/deno/releases/latest/download/deno-${architecture == 'arm64' ? 'aarch64' : 'x86_64'}-unknown-linux-gnu.zip';
    }
    throw '이 운영체제의 Deno 자동 설치 주소가 없습니다.';
  }

  Future<String> _machineArchitecture() async {
    if (Platform.isWindows) {
      final value =
          Platform.environment['PROCESSOR_ARCHITEW6432'] ??
          Platform.environment['PROCESSOR_ARCHITECTURE'] ??
          '';
      return value.toLowerCase().contains('arm') ? 'arm64' : 'x64';
    }
    try {
      final result = await Process.run('uname', <String>['-m']);
      final value = '${result.stdout}'.trim().toLowerCase();
      return value.contains('arm') || value.contains('aarch') ? 'arm64' : 'x64';
    } catch (_) {
      return 'x64';
    }
  }

  String get _ytdlpUrl => Platform.isWindows
      ? 'https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe'
      : Platform.isMacOS
      ? 'https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos'
      : 'https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp';

  String get _windowsFfmpegUrl =>
      'https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip';

  String get _macFfmpegUrl => 'https://evermeet.cx/ffmpeg/getrelease/zip';

  Future<String> _linuxFfmpegUrl() async {
    final architecture = await _machineArchitecture();
    final suffix = architecture == 'arm64' ? 'arm64' : 'amd64';
    return 'https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-$suffix-static.tar.xz';
  }

  String get ytdlpExecutable {
    final localName = Platform.isWindows ? 'yt-dlp.exe' : 'yt-dlp';
    final local = File(p.join(binDirectory.path, localName));
    return local.existsSync() ? local.path : localName;
  }

  String get ffmpegExecutable {
    final localName = Platform.isWindows ? 'ffmpeg.exe' : 'ffmpeg';
    final local = File(p.join(binDirectory.path, localName));
    return local.existsSync() ? local.path : localName;
  }

  String get denoExecutable {
    final localName = Platform.isWindows ? 'deno.exe' : 'deno';
    final local = File(p.join(binDirectory.path, localName));
    return local.existsSync() ? local.path : localName;
  }

  Future<PlaylistInfo> analyzeUrl({
    required String url,
    required AppSettings settings,
  }) async {
    final args = <String>[
      '--flat-playlist',
      '-J',
      '--no-warnings',
      '--socket-timeout',
      '30',
      ..._cookieArguments(settings),
      ..._runtimeArguments,
      url,
    ];
    final result = await Process.run(
      ytdlpExecutable,
      args,
      environment: _environment,
      runInShell: true,
    );
    if (result.exitCode != 0) {
      throw _formatAnalysisError('${result.stderr}');
    }
    try {
      final payload = jsonDecode('${result.stdout}');
      if (payload is! Map<String, dynamic>) {
        throw const FormatException('response is not an object');
      }
      return PlaylistInfo.fromJson(payload, sourceUrl: url);
    } on FormatException catch (error) {
      throw '영상 정보 JSON을 읽지 못했습니다: $error';
    }
  }

  Future<SupportedSites> supportedSites() async {
    final result = await Process.run(
      ytdlpExecutable,
      const <String>['--list-extractors'],
      environment: _environment,
      runInShell: true,
    );
    if (result.exitCode != 0) {
      final details = '${result.stderr}'.trim();
      throw details.isEmpty
          ? 'yt-dlp extractor 목록을 가져오지 못했습니다.'
          : 'yt-dlp extractor 목록을 가져오지 못했습니다: $details';
    }

    final extractors = _parseExtractorList('${result.stdout}');
    if (extractors.isEmpty) {
      throw 'yt-dlp extractor 목록이 비어 있습니다.';
    }
    return SupportedSites(extractors: extractors);
  }

  List<SupportedSite> _parseExtractorList(String output) {
    const brokenMarker = ' (CURRENTLY BROKEN)';
    final seen = <String>{};
    final extractors = <SupportedSite>[];
    for (final line in output.split(RegExp(r'\r?\n'))) {
      final raw = line.trim();
      if (raw.isEmpty) continue;
      final broken = raw.endsWith(brokenMarker);
      final name = broken
          ? raw.substring(0, raw.length - brokenMarker.length).trim()
          : raw;
      if (name.isEmpty || !seen.add(name)) continue;
      extractors.add(
        SupportedSite(name, broken ? SiteStatus.broken : SiteStatus.stable),
      );
    }
    return extractors;
  }

  /// Starts a sequential queue and emits live yt-dlp progress events.
  Stream<DownloadEvent> startDownload({
    required List<VideoEntry> entries,
    required AppSettings settings,
  }) {
    final controller = StreamController<DownloadEvent>();
    _stopRequested = false;
    unawaited(_downloadQueue(entries, settings, controller));
    return controller.stream;
  }

  Future<void> stopDownload() async {
    _stopRequested = true;
    final process = _downloadProcess;
    if (process == null) return;

    if (Platform.isWindows) {
      try {
        final result = await Process.run('taskkill', <String>[
          '/PID',
          '${process.pid}',
          '/T',
          '/F',
        ], runInShell: true);
        if (result.exitCode != 0) process.kill(ProcessSignal.sigterm);
      } catch (_) {
        process.kill(ProcessSignal.sigterm);
      }
    } else {
      // yt-dlp can have an ffmpeg child; stop both so a long encode cannot
      // continue after the user presses ABORT.
      try {
        await Process.run('pkill', <String>[
          '-TERM',
          '-P',
          '${process.pid}',
        ], runInShell: true);
      } catch (_) {
        // Some minimal environments do not ship pkill.
      }
      process.kill(ProcessSignal.sigterm);
    }
    try {
      await process.exitCode.timeout(const Duration(seconds: 2));
    } on TimeoutException {
      process.kill(ProcessSignal.sigkill);
    }
  }

  Future<void> openFolder(String path) async {
    final directory = Directory(path);
    if (!await directory.exists()) throw '저장 폴더가 존재하지 않습니다.';
    if (Platform.isWindows) {
      await Process.start('explorer', <String>[
        directory.path,
      ], runInShell: true);
    } else if (Platform.isMacOS) {
      await Process.start('open', <String>[directory.path]);
    } else {
      await Process.start('xdg-open', <String>[directory.path]);
    }
  }

  Future<void> _downloadQueue(
    List<VideoEntry> entries,
    AppSettings settings,
    StreamController<DownloadEvent> controller,
  ) async {
    try {
      if (entries.isEmpty) throw '다운로드할 영상을 선택해 주세요.';
      final outputDirectory = Directory(settings.downloadDir ?? '');
      if (!await outputDirectory.exists()) throw '저장 폴더를 먼저 선택해 주세요.';

      for (var index = 0; index < entries.length; index += 1) {
        if (_stopRequested) {
          controller.add(
            _event(
              'stopped',
              index + 1,
              entries.length,
              0,
              entries[index],
              '전체 작업을 취소했습니다.',
            ),
          );
          return;
        }
        final entry = entries[index];
        final itemNumber = index + 1;
        controller.add(
          _event(
            'starting',
            itemNumber,
            entries.length,
            _overall(index, 0, entries.length),
            entry,
            '다운로드를 시작합니다.',
          ),
        );
        final result = await _downloadEntry(
          entry,
          itemNumber,
          entries.length,
          settings,
          controller,
        );
        if (!result) return;
      }
      controller.add(
        DownloadEvent(
          kind: 'all_completed',
          current: entries.length,
          total: entries.length,
          percent: 100,
          title: '다운로드 완료',
          message: '모든 미디어를 저장했습니다.',
        ),
      );
    } catch (error) {
      controller.add(
        DownloadEvent(
          kind: 'failed',
          current: 1,
          total: entries.length,
          percent: 0,
          title: '다운로드 오류',
          message: '$error',
        ),
      );
    } finally {
      _downloadProcess = null;
      await controller.close();
    }
  }

  Future<bool> _downloadEntry(
    VideoEntry entry,
    int current,
    int total,
    AppSettings settings,
    StreamController<DownloadEvent> controller,
  ) async {
    final args = <String>[
      '--no-playlist',
      '--newline',
      '--progress',
      '--socket-timeout',
      '30',
      '--continue',
      '--retries',
      '10',
      '--fragment-retries',
      '10',
      '--file-access-retries',
      '5',
      '--trim-filenames',
      '120',
      '--add-metadata',
      '-o',
      p.join(settings.downloadDir!, '%(title)s.%(ext)s'),
      ..._cookieArguments(settings),
      ..._runtimeArguments,
    ];
    if (settings.format.isAudio) {
      args.addAll(<String>['-x', '--audio-format', settings.format.value]);
      if (settings.format == DownloadFormat.mp3) {
        args.addAll(<String>['--audio-quality', settings.audioQuality]);
      }
      // Keep the source resolution, but center-crop it to a square before
      // embedding so music players receive a proper album-art cover.
      args.addAll(<String>[
        '--embed-thumbnail',
        '--convert-thumbnails',
        'jpg',
        '--ppa',
        r'ThumbnailsConvertor+ffmpeg_o:-vf crop=min(iw\\,ih):min(iw\\,ih) -q:v 1',
      ]);
    } else if (settings.format == DownloadFormat.mp4) {
      args.addAll(<String>[
        '-f',
        'bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best',
        '--merge-output-format',
        'mp4',
      ]);
    } else {
      args.addAll(<String>[
        '-f',
        'bestvideo[ext=webm]+bestaudio/best',
        '--merge-output-format',
        'webm',
      ]);
    }
    args.add(entry.url);

    Process process;
    try {
      process = await Process.start(
        ytdlpExecutable,
        args,
        environment: _environment,
        runInShell: true,
      );
    } catch (error) {
      controller.add(
        _event('failed', current, total, 0, entry, 'yt-dlp 실행 실패: $error'),
      );
      return false;
    }
    _downloadProcess = process;

    final stderrLines = <String>[];
    var lastProgressAt = DateTime.fromMillisecondsSinceEpoch(0);
    var lastProgress = -1.0;
    var latestItemPercent = 0.0;
    void emitProcessLine(String line) {
      final match = RegExp(r'(\d+(?:\.\d+)?)%').firstMatch(line);
      final percent = match == null ? null : double.tryParse(match.group(1)!);
      if (percent != null) {
        final now = DateTime.now();
        final tooSoon =
            now.difference(lastProgressAt) < const Duration(milliseconds: 120);
        final barelyChanged = (percent - lastProgress).abs() < 0.5;
        if (tooSoon && barelyChanged && percent < 100) return;
        lastProgressAt = now;
        lastProgress = percent;
        latestItemPercent = percent;
      }
      _handleProcessLine(
        line,
        current,
        total,
        entry,
        controller,
        latestItemPercent,
        match != null,
      );
    }

    final stdoutSubscription = process.stdout
        .transform(utf8.decoder)
        .transform(const LineSplitter())
        .listen(emitProcessLine);
    final stderrSubscription = process.stderr
        .transform(utf8.decoder)
        .transform(const LineSplitter())
        .listen((line) {
          if (line.trim().isNotEmpty) {
            stderrLines.add(line.trim());
            if (stderrLines.length > 20) stderrLines.removeAt(0);
            emitProcessLine(line);
          }
        });
    final exitCode = await process.exitCode;
    await stdoutSubscription.cancel();
    await stderrSubscription.cancel();
    _downloadProcess = null;

    if (_stopRequested) {
      controller.add(
        _event(
          'stopped',
          current,
          total,
          _overall(current - 1, 0, total),
          entry,
          '전체 작업을 취소했습니다.',
        ),
      );
      return false;
    }
    if (exitCode != 0) {
      final details = stderrLines.isEmpty
          ? 'yt-dlp가 오류 코드 $exitCode로 종료되었습니다.'
          : stderrLines.last;
      controller.add(_event('failed', current, total, 0, entry, details));
      return false;
    }
    controller.add(
      _event(
        'completed',
        current,
        total,
        _overall(current - 1, 100, total),
        entry,
        '저장 완료',
      ),
    );
    return true;
  }

  void _handleProcessLine(
    String line,
    int current,
    int total,
    VideoEntry entry,
    StreamController<DownloadEvent> controller,
    double latestItemPercent,
    bool hasProgress,
  ) {
    final trimmed = line.trim();
    if (trimmed.isEmpty) return;
    final match = RegExp(r'(\d+(?:\.\d+)?)%').firstMatch(trimmed);
    final percent = hasProgress && match != null
        ? double.tryParse(match.group(1)!) ?? latestItemPercent
        : latestItemPercent;
    controller.add(
      _event(
        hasProgress ? 'progress' : 'message',
        current,
        total,
        _overall(current - 1, percent, total),
        entry,
        trimmed,
      ),
    );
  }

  DownloadEvent _event(
    String kind,
    int current,
    int total,
    double percent,
    VideoEntry entry,
    String message,
  ) {
    return DownloadEvent(
      kind: kind,
      current: current,
      total: total,
      percent: percent,
      title: entry.title,
      source: entry.source,
      message: message,
    );
  }

  double _overall(int index, double itemPercent, int total) {
    if (total == 0) return 0;
    return ((index + (itemPercent / 100)) / total) * 100;
  }

  List<String> _cookieArguments(AppSettings settings) {
    if (settings.cookieFile?.trim().isNotEmpty == true) {
      return <String>['--cookies', settings.cookieFile!];
    }
    if (settings.cookieBrowser != 'none') {
      return <String>['--cookies-from-browser', settings.cookieBrowser];
    }
    return const <String>[];
  }

  List<String> get _runtimeArguments {
    final deno = File(denoExecutable);
    if (!deno.existsSync()) return const <String>[];
    return <String>['--js-runtimes', 'deno:${deno.path}'];
  }

  Map<String, String> get _environment {
    final environment = Map<String, String>.from(Platform.environment);
    final localPath = binDirectory.path;
    final currentPath = environment['PATH'] ?? '';
    environment['PATH'] = [
      localPath,
      currentPath,
      if (Platform.isMacOS) '/opt/homebrew/bin',
      if (Platform.isMacOS) '/usr/local/bin',
    ].where((part) => part.isNotEmpty).join(Platform.pathSeparator);
    environment['PYTHONIOENCODING'] = 'utf-8';
    environment['PYTHONUTF8'] = '1';
    return environment;
  }

  Future<bool> _commandWorks(String executable, List<String> args) async {
    try {
      final result = await Process.run(
        executable,
        args,
        environment: _environment,
        runInShell: true,
      );
      return result.exitCode == 0;
    } catch (_) {
      return false;
    }
  }

  String _formatAnalysisError(String raw) {
    final message = raw.trim();
    if (message.isEmpty) return '영상 정보를 가져오지 못했습니다.';
    final lowered = message.toLowerCase();
    if (lowered.contains('sign in to confirm your age') ||
        lowered.contains('--cookies-from-browser')) {
      return '로그인이 필요한 영상입니다. 브라우저 쿠키 또는 cookies.txt를 설정해 주세요.';
    }
    return '영상 정보를 가져오지 못했습니다: $message';
  }
}
