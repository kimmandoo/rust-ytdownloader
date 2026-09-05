import 'package:flutter/material.dart';

import 'models/media_models.dart';
import 'state/app_controller.dart';
import 'widgets/pixel_widgets.dart';

class SpullApp extends StatefulWidget {
  const SpullApp({super.key, this.controller});

  final SpullController? controller;

  @override
  State<SpullApp> createState() => _SpullAppState();
}

class _SpullAppState extends State<SpullApp> {
  late final SpullController controller =
      widget.controller ?? SpullController();

  @override
  void initState() {
    super.initState();
    controller.boot();
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Spull',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        brightness: Brightness.light,
        scaffoldBackgroundColor: PixelColors.background,
        colorScheme: const ColorScheme.light(
          primary: PixelColors.orange,
          secondary: PixelColors.mint,
          surface: PixelColors.panel,
          onSurface: PixelColors.text,
        ),
        fontFamily: 'monospace',
        useMaterial3: true,
        inputDecorationTheme: const InputDecorationTheme(
          filled: true,
          fillColor: PixelColors.panelLight,
          border: OutlineInputBorder(
            borderSide: BorderSide(color: PixelColors.outline, width: 2),
          ),
          enabledBorder: OutlineInputBorder(
            borderSide: BorderSide(color: PixelColors.outline, width: 2),
          ),
          focusedBorder: OutlineInputBorder(
            borderSide: BorderSide(color: PixelColors.mint, width: 2),
          ),
          hintStyle: TextStyle(color: PixelColors.muted),
          contentPadding: EdgeInsets.symmetric(horizontal: 12, vertical: 12),
        ),
        checkboxTheme: CheckboxThemeData(
          fillColor: WidgetStateProperty.resolveWith(
            (states) => states.contains(WidgetState.selected)
                ? PixelColors.mint
                : PixelColors.panel,
          ),
          checkColor: WidgetStateProperty.all(PixelColors.ink),
          side: const BorderSide(color: PixelColors.outline, width: 2),
          shape: const BeveledRectangleBorder(),
        ),
      ),
      home: AnimatedBuilder(
        animation: controller,
        builder: (context, _) => _SpullDashboard(controller: controller),
      ),
    );
  }

  @override
  void dispose() {
    if (widget.controller == null) controller.dispose();
    super.dispose();
  }
}

class _SpullDashboard extends StatefulWidget {
  const _SpullDashboard({required this.controller});

  final SpullController controller;

  @override
  State<_SpullDashboard> createState() => _SpullDashboardState();
}

class _SpullDashboardState extends State<_SpullDashboard> {
  final Map<String, TextEditingController> _urlControllers =
      <String, TextEditingController>{};
  final TextEditingController _supportSearchController =
      TextEditingController();

  SpullController get controller => widget.controller;

  @override
  Widget build(BuildContext context) {
    _syncUrlControllers();
    return Scaffold(
      body: SafeArea(
        child: LayoutBuilder(
          builder: (context, constraints) {
            final compact = constraints.maxWidth < 980;
            final content = compact ? _compactLayout() : _wideLayout();
            return DecoratedBox(
              decoration: const BoxDecoration(
                color: PixelColors.background,
                backgroundBlendMode: BlendMode.srcOver,
              ),
              child: content,
            );
          },
        ),
      ),
    );
  }

  Widget _wideLayout() {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        SizedBox(width: 286, child: _buildSidebar()),
        Expanded(child: _buildWorkspace()),
      ],
    );
  }

  Widget _compactLayout() {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          _buildWorkspaceContent(),
          const SizedBox(height: 14),
          _buildSidebarSettings(),
        ],
      ),
    );
  }

  Widget _buildSidebar() {
    return Container(
      decoration: const BoxDecoration(
        color: PixelColors.panel,
        border: Border(right: BorderSide(color: PixelColors.outline, width: 2)),
      ),
      child: SingleChildScrollView(
        padding: const EdgeInsets.fromLTRB(18, 20, 18, 18),
        child: _buildSidebarSettings(),
      ),
    );
  }

  Widget _buildSidebarSettings() {
    final canEditSettings = controller.isReadyForInput && !controller.isBusy;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        _buildBrand(),
        const SizedBox(height: 18),
        PixelPanel(
          padding: const EdgeInsets.all(13),
          color: PixelColors.panelLight,
          child: Row(
            children: <Widget>[
              Container(
                width: 8,
                height: 8,
                color: controller.phase == AppPhase.downloading
                    ? PixelColors.orange
                    : PixelColors.mint,
              ),
              const SizedBox(width: 9),
              Expanded(
                child: Text(
                  _phaseLabel(),
                  style: const TextStyle(
                    color: PixelColors.text,
                    fontSize: 11,
                    fontWeight: FontWeight.w800,
                  ),
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 16),
        const _SectionLabel('OUTPUT FORMAT'),
        const SizedBox(height: 9),
        _buildFormatLoadout(),
        const SizedBox(height: 10),
        PixelPanel(
          padding: EdgeInsets.zero,
          color: PixelColors.panelLight,
          child: Material(
            color: PixelColors.panelLight,
            child: Theme(
              data: Theme.of(context)
                  .copyWith(dividerColor: Colors.transparent),
              child: ExpansionTile(
                tilePadding: const EdgeInsets.symmetric(horizontal: 12),
                childrenPadding: const EdgeInsets.fromLTRB(12, 0, 12, 12),
                iconColor: PixelColors.mint,
                collapsedIconColor: PixelColors.muted,
                title: const Text(
                  'ADVANCED SETTINGS',
                  style: TextStyle(
                    color: PixelColors.text,
                    fontSize: 10,
                    fontWeight: FontWeight.w900,
                    letterSpacing: 0.8,
                  ),
                ),
                subtitle: Text(
                  '${controller.settings.ytdlpChannel.toUpperCase()} · ${controller.settings.cookieBrowser == 'none' ? 'NO COOKIES' : controller.settings.cookieBrowser.toUpperCase()}',
                  style: const TextStyle(color: PixelColors.muted, fontSize: 9),
                ),
                children: <Widget>[
                  _buildSelectField(
                    label: 'YT-DLP CHANNEL',
                    value: controller.settings.ytdlpChannel,
                    items: const <String>['stable', 'nightly', 'master'],
                    onChanged: canEditSettings
                        ? (value) {
                            if (value != null) controller.setChannel(value);
                          }
                        : null,
                  ),
                  const SizedBox(height: 10),
                  _buildSelectField(
                    label: 'BROWSER COOKIES',
                    value: controller.settings.cookieBrowser,
                    items: const <String>[
                      'none',
                      'chrome',
                      'edge',
                      'firefox',
                      'brave',
                      'chromium',
                      'opera',
                      'vivaldi',
                      'safari',
                      'whale',
                    ],
                    onChanged: canEditSettings
                        ? (value) {
                            if (value != null) {
                              controller.setCookieBrowser(value);
                            }
                          }
                        : null,
                  ),
                  const SizedBox(height: 10),
                  _buildCookieFileCard(),
                ],
              ),
            ),
          ),
        ),
        const SizedBox(height: 10),
        _buildSupportButton(),
        if (controller.supportPanelOpen) ...<Widget>[
          const SizedBox(height: 10),
          _buildSupportPanel(),
        ],
      ],
    );
  }

  Widget _buildWorkspace() {
    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(24, 20, 24, 28),
      child: _buildWorkspaceContent(),
    );
  }

  Widget _buildWorkspaceContent() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        if (MediaQuery.sizeOf(context).width < 980) _buildCompactHeader(),
        if (controller.errorMessage.isNotEmpty) ...<Widget>[
          _buildErrorBanner(),
          const SizedBox(height: 14),
        ],
        _buildFolderPanel(),
        const SizedBox(height: 16),
        _buildUrlPanel(),
        const SizedBox(height: 16),
        _buildQueuePanel(),
        const SizedBox(height: 16),
        _buildProgressPanel(),
      ],
    );
  }

  Widget _buildBrand() {
    return Row(
      children: <Widget>[
        const PixelLogo(size: 54),
        const SizedBox(width: 12),
        Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: const <Widget>[
            Text(
              'SPULL',
              style: TextStyle(
                color: PixelColors.text,
                fontSize: 22,
                fontWeight: FontWeight.w900,
                letterSpacing: 2,
              ),
            ),
            Text(
              'MEDIA DOWNLOADER',
              style: TextStyle(
                color: PixelColors.orange,
                fontSize: 9,
                fontWeight: FontWeight.w900,
                letterSpacing: 1.6,
              ),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildCompactHeader() {
    return Row(
      children: <Widget>[
        const PixelLogo(size: 42),
        const SizedBox(width: 10),
        const Expanded(
          child: Text(
            'SPULL // MEDIA DOWNLOADER',
            style: TextStyle(
              color: PixelColors.text,
              fontWeight: FontWeight.w900,
              letterSpacing: 1.2,
            ),
          ),
        ),
        PixelTag(
          label: _phaseShortLabel(),
          color: controller.phase == AppPhase.downloading
              ? PixelColors.orange
              : PixelColors.mint,
        ),
      ],
    );
  }

  Widget _buildFormatLoadout() {
    final canEdit = controller.isReadyForInput && !controller.isBusy;
    final format = controller.settings.format;
    return PixelPanel(
      padding: const EdgeInsets.all(9),
      color: PixelColors.panelLight,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Wrap(
            spacing: 6,
            runSpacing: 6,
            children: DownloadFormat.values.map((format) {
              final selected = controller.settings.format == format;
              final foreground = !canEdit
                  ? PixelColors.muted
                  : selected
                  ? PixelColors.ink
                  : PixelColors.text;
              return InkWell(
                onTap: canEdit ? () => controller.setFormat(format) : null,
                child: Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 7,
                  ),
                  decoration: BoxDecoration(
                    color: !canEdit
                        ? const Color(0xFFF0E1D8)
                        : selected
                        ? PixelColors.orange
                        : PixelColors.panel,
                    border: Border.all(
                      color: selected && canEdit
                          ? PixelColors.orange
                          : PixelColors.outline,
                      width: 1,
                    ),
                  ),
                  child: Text(
                    format.label,
                    style: TextStyle(
                      color: foreground,
                      fontSize: 10,
                      fontWeight: FontWeight.w900,
                    ),
                  ),
                ),
              );
            }).toList(),
          ),
          if (format == DownloadFormat.mp3) ...<Widget>[
            const SizedBox(height: 8),
            const Row(
              children: <Widget>[
                Icon(Icons.album_outlined, color: PixelColors.mint, size: 14),
                SizedBox(width: 6),
                Expanded(
                  child: Text(
                    '320K · 앨범아트 MP3 내부 통합',
                    style: TextStyle(
                      color: PixelColors.mint,
                      fontSize: 9,
                      fontWeight: FontWeight.w800,
                    ),
                  ),
                ),
              ],
            ),
          ],
        ],
      ),
    );
  }

  Widget _buildSelectField({
    required String label,
    required String value,
    required List<String> items,
    required ValueChanged<String?>? onChanged,
  }) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Text(
          label,
          style: const TextStyle(
            color: PixelColors.muted,
            fontSize: 9,
            fontWeight: FontWeight.w900,
            letterSpacing: 1,
          ),
        ),
        const SizedBox(height: 5),
        Container(
          decoration: BoxDecoration(
            color: PixelColors.panelLight,
            border: Border.all(color: PixelColors.outline, width: 1),
          ),
          child: DropdownButtonHideUnderline(
            child: DropdownButton<String>(
              value: items.contains(value) ? value : items.first,
              isExpanded: true,
              padding: const EdgeInsets.symmetric(horizontal: 9),
              dropdownColor: PixelColors.panel,
              icon: const Icon(
                Icons.unfold_more,
                color: PixelColors.mint,
                size: 17,
              ),
              style: const TextStyle(
                color: PixelColors.text,
                fontFamily: 'monospace',
                fontSize: 11,
                fontWeight: FontWeight.w800,
              ),
              items: items
                  .map(
                    (item) => DropdownMenuItem<String>(
                      value: item,
                      child: Text(item.toUpperCase()),
                    ),
                  )
                  .toList(),
              onChanged: onChanged,
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildCookieFileCard() {
    final cookieFile = controller.settings.cookieFile;
    final canEdit = controller.isReadyForInput && !controller.isBusy;
    return PixelPanel(
      padding: const EdgeInsets.all(11),
      color: PixelColors.panelLight,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          const Text(
            'COOKIE FILE',
            style: TextStyle(
              color: PixelColors.muted,
              fontSize: 9,
              fontWeight: FontWeight.w900,
              letterSpacing: 1,
            ),
          ),
          const SizedBox(height: 5),
          Text(
            cookieFile ?? 'cookies.txt 미선택',
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: const TextStyle(color: PixelColors.text, fontSize: 10),
          ),
          const SizedBox(height: 9),
          Row(
            children: <Widget>[
              Expanded(
                child: PixelButton(
                  label: 'SELECT',
                  icon: Icons.folder_open,
                  onPressed: canEdit ? controller.chooseCookieFile : null,
                  expand: true,
                ),
              ),
              const SizedBox(width: 7),
              PixelButton(
                label: 'CLEAR',
                onPressed: canEdit && cookieFile != null
                    ? controller.clearCookieFile
                    : null,
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildSupportButton() {
    return PixelButton(
      label: controller.supportPanelOpen
          ? 'CLOSE EXTRACTORS'
          : 'EXTRACTOR CATALOG',
      icon: Icons.list_alt_outlined,
      onPressed: controller.toggleSupportPanel,
      expand: true,
    );
  }

  Widget _buildSupportPanel() {
    if (controller.supportLoading) {
      return const PixelPanel(
        child: Text(
          'yt-dlp extractor 목록을 불러오는 중...',
          style: TextStyle(color: PixelColors.muted, fontSize: 11),
        ),
      );
    }
    if (controller.supportError.isNotEmpty) {
      return PixelPanel(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Text(
              controller.supportError,
              style: const TextStyle(
                color: PixelColors.text,
                fontSize: 10,
                height: 1.4,
              ),
            ),
            const SizedBox(height: 9),
            PixelButton(
              label: 'RETRY',
              icon: Icons.refresh,
              onPressed: controller.retrySupportedSites,
              expand: true,
            ),
          ],
        ),
      );
    }
    final sites = controller.sites;
    if (sites == null) return const SizedBox.shrink();
    final visible = controller.filteredExtractors;
    return PixelPanel(
      padding: const EdgeInsets.all(11),
      color: PixelColors.panelLight,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Row(
            children: <Widget>[
              const Expanded(
                child: Text(
                  'SUPPORTED EXTRACTORS',
                  style: TextStyle(
                    color: PixelColors.text,
                    fontSize: 11,
                    fontWeight: FontWeight.w900,
                  ),
                ),
              ),
              PixelTag(
                label: '${visible.length}/${sites.extractors.length}',
                color: PixelColors.sky,
              ),
            ],
          ),
          const SizedBox(height: 8),
          TextField(
            controller: _supportSearchController,
            onChanged: controller.updateSupportQuery,
            style: const TextStyle(fontSize: 11, color: PixelColors.text),
            decoration: const InputDecoration(
              hintText: 'extractor 검색',
              prefixIcon: Icon(Icons.search, size: 16),
            ),
          ),
          const SizedBox(height: 8),
          const Text(
            'AVAILABLE · CURRENTLY BROKEN',
            style: TextStyle(
              color: PixelColors.muted,
              fontSize: 9,
              fontWeight: FontWeight.w800,
              letterSpacing: 0.7,
            ),
          ),
          const SizedBox(height: 6),
          SizedBox(
            height: 320,
            child: visible.isEmpty
                ? const Center(
                    child: Text(
                      '검색 결과가 없습니다.',
                      style: TextStyle(color: PixelColors.muted, fontSize: 10),
                    ),
                  )
                : Scrollbar(
                    child: ListView.separated(
                      itemCount: visible.length,
                      separatorBuilder: (_, index) => const SizedBox(height: 4),
                      itemBuilder: (_, index) {
                        final site = visible[index];
                        final broken = site.status == SiteStatus.broken;
                        return Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 8,
                            vertical: 6,
                          ),
                          decoration: BoxDecoration(
                            color: PixelColors.panel,
                            border: Border.all(
                              color: broken
                                  ? PixelColors.pink
                                  : PixelColors.outline,
                              width: 1,
                            ),
                          ),
                          child: Row(
                            children: <Widget>[
                              Expanded(
                                child: Text(
                                  site.name,
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: const TextStyle(
                                    color: PixelColors.text,
                                    fontSize: 10,
                                    fontWeight: FontWeight.w700,
                                  ),
                                ),
                              ),
                              const SizedBox(width: 8),
                              PixelTag(
                                label: broken ? 'BROKEN' : 'AVAILABLE',
                                color: _siteColor(site.status),
                              ),
                            ],
                          ),
                        );
                      },
                    ),
                  ),
          ),
        ],
      ),
    );
  }

  Widget _buildErrorBanner() {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFFFFDCE2),
        border: Border.all(color: PixelColors.pink, width: 2),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          const Icon(
            Icons.warning_amber_rounded,
            color: PixelColors.pink,
            size: 18,
          ),
          const SizedBox(width: 9),
          Expanded(
            child: Text(
              controller.errorMessage,
              style: const TextStyle(
                color: PixelColors.text,
                fontSize: 11,
                height: 1.4,
              ),
            ),
          ),
          IconButton(
            tooltip: '오류 닫기',
            onPressed: controller.clearError,
            icon: const Icon(Icons.close, color: PixelColors.muted, size: 17),
          ),
        ],
      ),
    );
  }

  Widget _buildFolderPanel() {
    final path = controller.settings.downloadDir;
    final configured = path?.isNotEmpty == true;
    final canEdit = controller.isReadyForInput && !controller.isBusy;
    final displayPath = configured ? path! : '기본 Downloads 폴더를 준비 중입니다.';
    return PixelPanel(
      color: PixelColors.panelLight,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Row(
            children: <Widget>[
              Container(
                width: 30,
                height: 30,
                color: PixelColors.sky,
                alignment: Alignment.center,
                child: const Icon(
                  Icons.folder_outlined,
                  color: PixelColors.ink,
                  size: 18,
                ),
              ),
              const SizedBox(width: 10),
              const Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      'DOWNLOAD FOLDER',
                      style: TextStyle(
                        color: PixelColors.text,
                        fontSize: 11,
                        fontWeight: FontWeight.w900,
                        letterSpacing: 1,
                      ),
                    ),
                    SizedBox(height: 3),
                    Text(
                      '새 파일이 저장되는 위치',
                      style: TextStyle(color: PixelColors.muted, fontSize: 9),
                    ),
                  ],
                ),
              ),
              PixelTag(
                label: configured ? 'READY' : 'SETUP',
                color: configured ? PixelColors.sky : PixelColors.orange,
              ),
            ],
          ),
          const SizedBox(height: 12),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
            decoration: BoxDecoration(
              color: PixelColors.panel,
              border: Border.all(color: PixelColors.outline, width: 1),
            ),
            child: Row(
              children: <Widget>[
                const Icon(
                  Icons.folder_open,
                  color: PixelColors.orange,
                  size: 16,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    displayPath,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      color: PixelColors.text,
                      fontSize: 10,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 10),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            alignment: WrapAlignment.end,
            children: <Widget>[
              PixelButton(
                label: 'CHANGE FOLDER',
                icon: Icons.drive_file_move_outline,
                onPressed: canEdit ? controller.chooseFolder : null,
              ),
              if (configured)
                PixelButton(
                  label: 'OPEN',
                  icon: Icons.folder_open,
                  onPressed: controller.openFolder,
                ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildUrlPanel() {
    final canEdit = controller.isReadyForInput && !controller.isBusy;
    return PixelPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Row(
            children: <Widget>[
              const Expanded(
                child: _PanelTitle(kicker: 'LINKS', title: '링크를 넣어 주세요'),
              ),
              PixelTag(
                label: '${controller.urlRows.length} SLOTS',
                color: PixelColors.sky,
              ),
            ],
          ),
          const SizedBox(height: 14),
          ...controller.urlRows.asMap().entries.map(
            (entry) => _buildUrlRow(entry.key, entry.value),
          ),
          const SizedBox(height: 11),
          Wrap(
            spacing: 9,
            runSpacing: 9,
            alignment: WrapAlignment.spaceBetween,
            children: <Widget>[
              PixelButton(
                label: 'ADD SLOT',
                icon: Icons.add,
                onPressed: canEdit ? controller.addUrlRow : null,
              ),
              PixelButton(
                label: controller.phase == AppPhase.analyzing
                    ? 'SCANNING...'
                    : 'SCAN LINKS',
                icon: Icons.radar,
                tone: PixelButtonTone.primary,
                onPressed: canEdit ? controller.analyze : null,
              ),
            ],
          ),
          const SizedBox(height: 9),
          Text(
            controller.urlRows.any((row) => row.value.trim().isNotEmpty)
                ? '플레이리스트는 아래에서 필요한 파일만 골라 주세요.'
                : 'YouTube 안정 지원 · 링크를 스캔해 다운로드 목록 만들기',
            style: const TextStyle(color: PixelColors.muted, fontSize: 10),
          ),
        ],
      ),
    );
  }

  Widget _buildUrlRow(int index, UrlRow row) {
    final textController = _urlControllers[row.id]!;
    return Padding(
      padding: EdgeInsets.only(
        bottom: index == controller.urlRows.length - 1 ? 0 : 8,
      ),
      child: Row(
        children: <Widget>[
          Container(
            width: 30,
            height: 48,
            alignment: Alignment.center,
            color: PixelColors.panelLight,
            child: Text(
              (index + 1).toString().padLeft(2, '0'),
              style: const TextStyle(
                color: PixelColors.orange,
                fontSize: 10,
                fontWeight: FontWeight.w900,
              ),
            ),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: TextField(
              controller: textController,
              enabled: controller.isReadyForInput && !controller.isBusy,
              onChanged: (value) => controller.updateUrl(row.id, value),
              onSubmitted: (_) => controller.analyze(),
              style: const TextStyle(color: PixelColors.text, fontSize: 12),
              decoration: const InputDecoration(
                hintText: 'https://www.youtube.com/watch?v=...',
              ),
            ),
          ),
          const SizedBox(width: 8),
          IconButton(
            tooltip: '슬롯 삭제',
            onPressed:
                controller.urlRows.length == 1 ||
                    controller.isBusy ||
                    !controller.isReadyForInput
                ? null
                : () => _removeUrlController(row.id),
            icon: const Icon(Icons.close, size: 17),
            color: PixelColors.pink,
          ),
        ],
      ),
    );
  }

  Widget _buildQueuePanel() {
    final playlist = controller.playlist;
    final canSelect = controller.isReadyForInput && !controller.isBusy;
    if (playlist == null) {
      return PixelPanel(
        color: PixelColors.panel,
        child: Row(
          children: <Widget>[
            Container(width: 7, height: 54, color: PixelColors.orange),
            const SizedBox(width: 15),
            const Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    '다운로드 목록이 비어 있어요',
                    style: TextStyle(
                      color: PixelColors.orange,
                      fontSize: 11,
                      fontWeight: FontWeight.w900,
                      letterSpacing: 1,
                    ),
                  ),
                  SizedBox(height: 5),
                  Text(
                    '링크를 분석하면 다운로드할 파일이 여기에 나타나요.',
                    style: TextStyle(color: PixelColors.muted, fontSize: 11),
                  ),
                ],
              ),
            ),
            Icon(
              Icons.inventory_2_outlined,
              color: PixelColors.outline,
              size: 30,
            ),
          ],
        ),
      );
    }
    return PixelPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Row(
            children: <Widget>[
              Expanded(
                child: _PanelTitle(
                  kicker: playlist.isPlaylist
                      ? 'DOWNLOAD QUEUE · PLAYLIST'
                      : 'DOWNLOAD QUEUE · SINGLE',
                  title: playlist.title,
                ),
              ),
              Text(
                '${controller.selectedCount}/${controller.totalCount}',
                style: const TextStyle(
                  color: PixelColors.mint,
                  fontWeight: FontWeight.w900,
                  fontSize: 12,
                ),
              ),
            ],
          ),
          const SizedBox(height: 11),
          Row(
            children: <Widget>[
              PixelButton(
                label: 'ALL',
                onPressed: canSelect ? () => controller.toggleAll(true) : null,
              ),
              const SizedBox(width: 7),
              PixelButton(
                label: 'NONE',
                onPressed: canSelect ? () => controller.toggleAll(false) : null,
              ),
            ],
          ),
          if (controller.selectedCount == 0)
            const Padding(
              padding: EdgeInsets.only(bottom: 10),
              child: Text(
                '다운로드할 항목을 하나 이상 선택하세요.',
                style: TextStyle(
                  color: PixelColors.yellow,
                  fontSize: 10,
                  fontWeight: FontWeight.w800,
                ),
              ),
            ),
          const SizedBox(height: 12),
          ...playlist.entries.asMap().entries.map(
            (entry) => _buildVideoRow(entry.key, entry.value),
          ),
        ],
      ),
    );
  }

  Widget _buildVideoRow(int index, VideoEntry entry) {
    final canSelect = controller.isReadyForInput && !controller.isBusy;
    return Container(
      margin: const EdgeInsets.only(bottom: 7),
      padding: const EdgeInsets.all(9),
      decoration: BoxDecoration(
        color: PixelColors.panelLight,
        border: Border.all(
          color: entry.selected ? PixelColors.orange : const Color(0xFFE4D5CF),
          width: 1,
        ),
      ),
      child: Row(
        children: <Widget>[
          Checkbox(
            value: entry.selected,
            onChanged: canSelect
                ? (value) => controller.toggleEntry(entry.id, value ?? false)
                : null,
          ),
          Container(
            width: 64,
            height: 42,
            color: PixelColors.panel,
            alignment: Alignment.center,
            child: entry.thumbnail == null
                ? Text(
                    (index + 1).toString().padLeft(2, '0'),
                    style: const TextStyle(
                      color: PixelColors.yellow,
                      fontWeight: FontWeight.w900,
                    ),
                  )
                : Image.network(
                    entry.thumbnail!,
                    fit: BoxFit.cover,
                    errorBuilder: (_, _, _) => Text(
                      (index + 1).toString(),
                      style: const TextStyle(color: PixelColors.yellow),
                    ),
                  ),
          ),
          const SizedBox(width: 11),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  '${index + 1}. ${entry.title}',
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    color: PixelColors.text,
                    fontWeight: FontWeight.w800,
                    fontSize: 11,
                    height: 1.3,
                  ),
                ),
                const SizedBox(height: 5),
                Row(
                  children: <Widget>[
                    Text(
                      entry.displayDuration,
                      style: const TextStyle(
                        color: PixelColors.muted,
                        fontSize: 10,
                      ),
                    ),
                    if (entry.source != null) ...<Widget>[
                      const SizedBox(width: 8),
                      PixelTag(label: entry.source!, color: PixelColors.sky),
                    ],
                  ],
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildProgressPanel() {
    final isDownloading = controller.phase == AppPhase.downloading;
    final heading = isDownloading
        ? '다운로드 중: ${controller.downloadTitle}'
        : controller.phase == AppPhase.finished
        ? '다운로드 완료'
        : '다운로드 대기';
    return PixelPanel(
      color: PixelColors.panelLight,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Row(
            children: <Widget>[
              Expanded(
                child: _PanelTitle(kicker: 'DOWNLOAD PROGRESS', title: heading),
              ),
              PixelTag(
                label: '${controller.downloadPercent.round()}%',
                color: isDownloading ? PixelColors.orange : PixelColors.mint,
              ),
            ],
          ),
          const SizedBox(height: 14),
          PixelProgressBar(value: controller.downloadPercent / 100),
          const SizedBox(height: 9),
          Row(
            children: <Widget>[
              Text(
                isDownloading
                    ? '${controller.downloadCurrent}/${controller.downloadTotal}'
                    : 'STANDBY',
                style: const TextStyle(
                  color: PixelColors.mint,
                  fontSize: 10,
                  fontWeight: FontWeight.w900,
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  controller.downloadMessage,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(color: PixelColors.text, fontSize: 10),
                ),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Row(
            children: <Widget>[
              Text(
                'ELAPSED ${controller.elapsedLabel}',
                style: const TextStyle(
                  color: PixelColors.muted,
                  fontSize: 9,
                  fontWeight: FontWeight.w800,
                ),
              ),
              const Spacer(),
              Text(
                'ETA ${controller.etaLabel}',
                style: const TextStyle(
                  color: PixelColors.muted,
                  fontSize: 9,
                  fontWeight: FontWeight.w800,
                ),
              ),
            ],
          ),
          const SizedBox(height: 13),
          if (controller.logs.isNotEmpty)
            Container(
              height: 74,
              padding: const EdgeInsets.all(9),
              color: PixelColors.panel,
              child: ListView.builder(
                reverse: true,
                itemCount: controller.logs.length,
                itemBuilder: (_, index) {
                  final log =
                      controller.logs[controller.logs.length - index - 1];
                  return Text(
                    log,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      color: PixelColors.muted,
                      fontSize: 9,
                      height: 1.5,
                    ),
                  );
                },
              ),
            ),
          if (controller.logs.isEmpty)
            const Text(
              '진행 메시지가 여기에 표시돼요.',
              style: TextStyle(color: PixelColors.muted, fontSize: 10),
            ),
          const SizedBox(height: 14),
          Wrap(
            spacing: 9,
            runSpacing: 9,
            alignment: WrapAlignment.end,
            children: <Widget>[
              if (isDownloading)
                PixelButton(
                  label: 'CANCEL',
                  icon: Icons.stop,
                  tone: PixelButtonTone.danger,
                  onPressed: controller.stopDownload,
                )
              else
                PixelButton(
                  label: '${controller.selectedCount} DOWNLOAD',
                  icon: Icons.download,
                  tone: PixelButtonTone.primary,
                  onPressed:
                      controller.readyToDownload &&
                          controller.hasDownloadableSelection
                      ? controller.startDownload
                      : null,
                ),
            ],
          ),
        ],
      ),
    );
  }

  void _syncUrlControllers() {
    final ids = controller.urlRows.map((row) => row.id).toSet();
    for (final id in _urlControllers.keys.toList()) {
      if (!ids.contains(id)) {
        _urlControllers.remove(id)?.dispose();
      }
    }
    for (final row in controller.urlRows) {
      _urlControllers.putIfAbsent(
        row.id,
        () => TextEditingController(text: row.value),
      );
    }
  }

  void _removeUrlController(String id) {
    _urlControllers.remove(id)?.dispose();
    controller.removeUrlRow(id);
  }

  String _phaseLabel() {
    return switch (controller.phase) {
      AppPhase.booting => controller.initMessage,
      AppPhase.idle => '분석 대기 중',
      AppPhase.analyzing => '링크 분석 중',
      AppPhase.ready => '다운로드 항목 선택 가능',
      AppPhase.downloading => '다운로드 중',
      AppPhase.finished => '모든 작업 완료',
    };
  }

  String _phaseShortLabel() {
    return switch (controller.phase) {
      AppPhase.booting => 'BOOT',
      AppPhase.idle => 'SCAN',
      AppPhase.analyzing => 'SCAN...',
      AppPhase.ready => 'READY',
      AppPhase.downloading => 'DOWN',
      AppPhase.finished => 'DONE',
    };
  }

  Color _siteColor(SiteStatus status) {
    return switch (status) {
      SiteStatus.stable => PixelColors.mint,
      SiteStatus.experimental => PixelColors.yellow,
      SiteStatus.broken => PixelColors.pink,
    };
  }

  @override
  void dispose() {
    for (final controller in _urlControllers.values) {
      controller.dispose();
    }
    _supportSearchController.dispose();
    super.dispose();
  }
}

class _SectionLabel extends StatelessWidget {
  const _SectionLabel(this.label);

  final String label;

  @override
  Widget build(BuildContext context) => Text(
    label,
    style: const TextStyle(
      color: PixelColors.muted,
      fontSize: 9,
      fontWeight: FontWeight.w900,
      letterSpacing: 1.8,
    ),
  );
}

class _PanelTitle extends StatelessWidget {
  const _PanelTitle({required this.kicker, required this.title});

  final String kicker;
  final String title;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Text(
          kicker,
          style: const TextStyle(
            color: PixelColors.orange,
            fontSize: 9,
            fontWeight: FontWeight.w900,
            letterSpacing: 1.4,
          ),
        ),
        const SizedBox(height: 4),
        Text(
          title,
          maxLines: 2,
          overflow: TextOverflow.ellipsis,
          style: const TextStyle(
            color: PixelColors.text,
            fontSize: 16,
            fontWeight: FontWeight.w900,
          ),
        ),
      ],
    );
  }
}
