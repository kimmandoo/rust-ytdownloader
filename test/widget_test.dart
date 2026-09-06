import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:spull/home_page.dart';
import 'package:spull/models/media_models.dart';
import 'package:spull/services/spull_backend.dart';
import 'package:spull/state/app_controller.dart';
import 'package:spull/widgets/pixel_widgets.dart';

void main() {
  testWidgets('renders the Spull dashboard', (tester) async {
    final controller = SpullController();
    await tester.pumpWidget(SpullApp(controller: controller));

    expect(find.text('MEDIA DOWNLOADER'), findsOneWidget);
    expect(find.text('LINKS'), findsOneWidget);
    expect(find.text('SCAN LINKS'), findsOneWidget);
    expect(find.text('DOWNLOAD FOLDER'), findsOneWidget);

    controller.dispose();
  });

  test('persists selected audio and video quality', () {
    const settings = AppSettings(
      format: DownloadFormat.mp4,
      audioQuality: '192K',
      videoQuality: '1080p',
    );

    final restored = AppSettings.fromJson(settings.toJson());

    expect(restored.audioQuality, '192K');
    expect(restored.videoQuality, '1080p');
  });

  test('uses safe quality defaults for unknown saved values', () {
    final settings = AppSettings.fromJson({
      'audio_quality': 'invalid',
      'video_quality': 'invalid',
    });

    expect(settings.audioQuality, '320K');
    expect(settings.videoQuality, 'best');
  });

  test('cancelling analysis returns the controller to idle', () async {
    final backend = _BlockingBackend();
    final controller = SpullController(backend: backend);
    controller.urlRows.first.value = 'https://example.com/video';

    final analysis = controller.analyze();
    expect(controller.phase, AppPhase.analyzing);
    await controller.cancelAnalyze();
    await analysis;

    expect(backend.analysisCancelled, isTrue);
    expect(controller.phase, AppPhase.idle);
    expect(controller.logs.last, '분석을 취소했습니다.');
    controller.dispose();
  });

  test('cancelling extractor loading clears its loading state', () async {
    final backend = _BlockingBackend();
    final controller = SpullController(backend: backend);

    final loading = controller.toggleSupportPanel();
    await Future<void>.delayed(Duration.zero);
    expect(controller.supportLoading, isTrue);

    await controller.cancelSupportedSites();
    await loading;

    expect(backend.supportSitesCancelled, isTrue);
    expect(controller.supportLoading, isFalse);
    expect(controller.supportError, 'extractor 목록 로딩을 취소했습니다.');
    controller.dispose();
  });

  testWidgets('renders an indeterminate pixel progress bar', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(home: Scaffold(body: PixelProgressBar(value: null))),
    );

    expect(find.byType(LinearProgressIndicator), findsOneWidget);
  });
}

class _BlockingBackend extends SpullBackend {
  final Completer<PlaylistInfo> _analysis = Completer<PlaylistInfo>();
  final Completer<SupportedSites> _supportedSites = Completer<SupportedSites>();
  bool analysisCancelled = false;
  bool supportSitesCancelled = false;
  bool analysisStarted = false;
  bool supportSitesStarted = false;

  @override
  Future<PlaylistInfo> analyzeUrl({
    required String url,
    required AppSettings settings,
  }) {
    analysisStarted = true;
    return _analysis.future;
  }

  @override
  Future<void> cancelAnalysis() async {
    analysisCancelled = true;
    if (analysisStarted && !_analysis.isCompleted) {
      _analysis.completeError(const SpullOperationCancelled('링크 분석'));
    }
  }

  @override
  Future<SupportedSites> supportedSites() {
    supportSitesStarted = true;
    return _supportedSites.future;
  }

  @override
  Future<void> cancelSupportedSites() async {
    supportSitesCancelled = true;
    if (supportSitesStarted && !_supportedSites.isCompleted) {
      _supportedSites.completeError(
        const SpullOperationCancelled('extractor 목록 조회'),
      );
    }
  }
}
