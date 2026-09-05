import 'package:flutter_test/flutter_test.dart';

import 'package:spull/home_page.dart';
import 'package:spull/state/app_controller.dart';

void main() {
  testWidgets('renders the Spull dashboard', (tester) async {
    final controller = SpullController();
    await tester.pumpWidget(SpullApp(controller: controller));

    expect(find.text('MEDIA DOWNLOADER'), findsOneWidget);
    expect(find.text('LINKS'), findsOneWidget);
    expect(find.text('SCAN LINKS'), findsOneWidget);

    controller.dispose();
  });
}
