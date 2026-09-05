import 'dart:io';

const supportedTargets = <String>{'linux', 'macos', 'windows'};

Future<void> main(List<String> arguments) async {
  if (arguments.length != 1 || !supportedTargets.contains(arguments.single)) {
    stderr.writeln(
      'Usage: dart run tool/verify_desktop_artifact.dart <linux|macos|windows>',
    );
    exitCode = 64;
    return;
  }

  final target = arguments.single;
  final requiredPaths = switch (target) {
    'linux' => <String>[
      'build/linux/x64/release/bundle/spull',
      'build/linux/x64/release/bundle/data/app.so',
      'build/linux/x64/release/bundle/data/icudtl.dat',
      'build/linux/x64/release/bundle/data/flutter_assets/AssetManifest.bin',
      'build/linux/x64/release/bundle/lib/libflutter_linux_gtk.so',
    ],
    'macos' => <String>[
      'build/macos/Build/Products/Release/spull.app/Contents/MacOS/spull',
      'build/macos/Build/Products/Release/spull.app/Contents/Frameworks/App.framework/App',
      'build/macos/Build/Products/Release/spull.app/Contents/Frameworks/FlutterMacOS.framework/FlutterMacOS',
      'build/macos/Build/Products/Release/spull.app/Contents/Frameworks/App.framework/Resources/flutter_assets',
    ],
    'windows' => <String>[
      'build/windows/x64/runner/Release/spull.exe',
      'build/windows/x64/runner/Release/flutter_windows.dll',
      'build/windows/x64/runner/Release/data/app.so',
      'build/windows/x64/runner/Release/data/icudtl.dat',
      'build/windows/x64/runner/Release/data/flutter_assets/AssetManifest.bin',
    ],
    _ => const <String>[],
  };

  for (final relativePath in requiredPaths) {
    final entity = FileSystemEntity.typeSync(relativePath);
    if (entity == FileSystemEntityType.notFound) {
      throw StateError('Missing $target runtime dependency: $relativePath');
    }
    if (entity == FileSystemEntityType.file &&
        File(relativePath).lengthSync() == 0) {
      throw StateError('Empty $target runtime dependency: $relativePath');
    }
    if (entity == FileSystemEntityType.directory &&
        Directory(relativePath).listSync(followLinks: false).isEmpty) {
      throw StateError('Empty $target runtime directory: $relativePath');
    }
  }

  stdout.writeln('$target desktop runtime verification passed.');
  for (final relativePath in requiredPaths) {
    final entity = FileSystemEntity.typeSync(relativePath);
    final size = entity == FileSystemEntityType.file
        ? File(relativePath).lengthSync()
        : Directory(relativePath).listSync(followLinks: false).length;
    stdout.writeln('  $relativePath ($size)');
  }
}
