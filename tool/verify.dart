import 'dart:io';

/// Runs the checks shared by local development and GitHub Actions.
Future<void> main() async {
  final flutter = Platform.isWindows ? 'flutter.bat' : 'flutter';

  await runCommand(flutter, ['pub', 'get']);
  await runCommand('dart', [
    'format',
    '--output=none',
    '--set-exit-if-changed',
    'lib',
    'test',
    'tool',
  ]);
  await runCommand(flutter, ['analyze']);
  await runCommand(flutter, ['test', '--concurrency=1', '--timeout=2m']);
}

Future<void> runCommand(String executable, List<String> arguments) async {
  stdout.writeln('\n> $executable ${arguments.join(' ')}');
  final result = await Process.run(
    executable,
    arguments,
    runInShell: Platform.isWindows,
  );
  stdout.write(result.stdout);
  stderr.write(result.stderr);
  if (result.exitCode != 0) {
    exit(result.exitCode);
  }
}
