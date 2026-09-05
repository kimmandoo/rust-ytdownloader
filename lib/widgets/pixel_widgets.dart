import 'package:flutter/material.dart';

class PixelColors {
  static const ink = Color(0xFF17162A);
  static const background = Color(0xFFECE3D8);
  static const panel = Color(0xFFE2D7CC);
  static const panelLight = Color(0xFFF4ECE2);
  static const outline = Color(0xFFB47A6E);
  static const cream = Color(0xFFFFF4D1);
  static const text = Color(0xFF2A2037);
  static const muted = Color(0xFF766B78);
  static const orange = Color(0xFFE28A66);
  static const yellow = Color(0xFFD4AD55);
  static const mint = Color(0xFF69B89F);
  static const pink = Color(0xFFD86D82);
  static const sky = Color(0xFFCE9A82);
  static const logoOrange = Color(0xFFFF9F68);
  static const logoMint = Color(0xFF7EE6C4);
  static const logoPink = Color(0xFFFF7891);
}

class PixelPanel extends StatelessWidget {
  const PixelPanel({
    super.key,
    required this.child,
    this.padding = const EdgeInsets.all(18),
    this.color = PixelColors.panel,
  });

  final Widget child;
  final EdgeInsets padding;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: color,
        border: Border.all(color: PixelColors.outline, width: 1),
        boxShadow: const <BoxShadow>[
          BoxShadow(
            color: PixelColors.outline,
            offset: Offset(2, 2),
            blurRadius: 0,
          ),
        ],
      ),
      child: Padding(padding: padding, child: child),
    );
  }
}

class PixelButton extends StatelessWidget {
  const PixelButton({
    super.key,
    required this.label,
    required this.onPressed,
    this.icon,
    this.tone = PixelButtonTone.ghost,
    this.expand = false,
  });

  final String label;
  final VoidCallback? onPressed;
  final IconData? icon;
  final PixelButtonTone tone;
  final bool expand;

  @override
  Widget build(BuildContext context) {
    final enabled = onPressed != null;
    final baseBackground = switch (tone) {
      PixelButtonTone.primary => PixelColors.orange,
      PixelButtonTone.danger => PixelColors.pink,
      PixelButtonTone.ghost => PixelColors.panelLight,
    };
    final background = enabled ? baseBackground : const Color(0xFFE9D8D0);
    final foreground = enabled
        ? (tone == PixelButtonTone.ghost ? PixelColors.text : PixelColors.ink)
        : PixelColors.muted;
    final button = Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: onPressed,
        child: Ink(
          color: background,
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 11),
          child: Row(
            mainAxisSize: expand ? MainAxisSize.max : MainAxisSize.min,
            mainAxisAlignment: MainAxisAlignment.center,
            children: <Widget>[
              if (icon != null) ...<Widget>[
                Icon(icon, size: 16, color: foreground),
                const SizedBox(width: 7),
              ],
              Text(
                label,
                style: TextStyle(
                  color: foreground,
                  fontWeight: FontWeight.w900,
                  fontSize: 12,
                  letterSpacing: 0.5,
                ),
              ),
            ],
          ),
        ),
      ),
    );
    return Semantics(
      button: true,
      enabled: enabled,
      label: label,
      child: Container(
        decoration: BoxDecoration(
          border: Border.all(
            color: enabled && tone != PixelButtonTone.ghost
                ? baseBackground
                : PixelColors.outline,
            width: 1,
          ),
          boxShadow: enabled
              ? const <BoxShadow>[
                  BoxShadow(
                    color: PixelColors.outline,
                    offset: Offset(1, 1),
                    blurRadius: 0,
                  ),
                ]
              : null,
        ),
        child: button,
      ),
    );
  }
}

enum PixelButtonTone { primary, danger, ghost }

class PixelTag extends StatelessWidget {
  const PixelTag({
    super.key,
    required this.label,
    this.color = PixelColors.mint,
  });

  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 4),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.16),
        border: Border.all(color: color, width: 1),
      ),
      child: Text(
        label,
        style: TextStyle(
          color: color,
          fontSize: 10,
          fontWeight: FontWeight.w900,
          letterSpacing: 0.8,
        ),
      ),
    );
  }
}

class PixelLogo extends StatelessWidget {
  const PixelLogo({super.key, this.size = 58});

  final double size;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: size,
      height: size,
      child: CustomPaint(painter: _PixelLogoPainter()),
    );
  }
}

class PixelProgressBar extends StatelessWidget {
  const PixelProgressBar({super.key, required this.value, this.height = 14});
  final double value;
  final double height;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: height,
      decoration: BoxDecoration(
        color: PixelColors.panelLight,
        border: Border.all(color: PixelColors.outline, width: 2),
      ),
      padding: const EdgeInsets.all(3),
      child: Align(
        alignment: Alignment.centerLeft,
        child: FractionallySizedBox(
          widthFactor: value.clamp(0, 1),
          child: Container(
            decoration: const BoxDecoration(color: PixelColors.mint),
            child: Row(
              children: List<Widget>.generate(
                12,
                (_) => const Expanded(child: SizedBox()),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class PixelScene extends StatelessWidget {
  const PixelScene({super.key});

  @override
  Widget build(BuildContext context) {
    return CustomPaint(
      painter: _PixelScenePainter(),
      child: const SizedBox.expand(),
    );
  }
}

class _PixelLogoPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final scale = size.shortestSide / 16;
    final fill = Paint()..style = PaintingStyle.fill;
    void block(int x, int y, int width, int height, Color color) {
      fill.color = color;
      canvas.drawRect(
        Rect.fromLTWH(x * scale, y * scale, width * scale, height * scale),
        fill,
      );
    }

    // Transparent pixel logo mark.
    block(3, 0, 3, 1, PixelColors.ink);
    block(10, 0, 3, 1, PixelColors.ink);
    block(2, 1, 5, 4, PixelColors.ink);
    block(9, 1, 5, 4, PixelColors.ink);
    block(1, 4, 14, 9, PixelColors.ink);
    block(3, 13, 10, 2, PixelColors.ink);
    block(3, 2, 3, 3, PixelColors.logoOrange);
    block(10, 2, 3, 3, PixelColors.logoOrange);
    block(2, 5, 12, 7, PixelColors.logoOrange);
    block(4, 7, 2, 2, PixelColors.ink);
    block(10, 7, 2, 2, PixelColors.ink);
    block(5, 7, 1, 1, PixelColors.cream);
    block(10, 7, 1, 1, PixelColors.cream);
    block(2, 9, 2, 1, PixelColors.cream);
    block(12, 9, 2, 1, PixelColors.cream);
    block(5, 9, 6, 3, PixelColors.cream);
    block(7, 10, 2, 1, PixelColors.logoPink);
    block(6, 11, 1, 1, PixelColors.ink);
    block(9, 11, 1, 1, PixelColors.ink);
    block(5, 13, 6, 1, PixelColors.logoMint);
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

class _PixelScenePainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()..style = PaintingStyle.fill;
    paint.color = PixelColors.panel;
    canvas.drawRect(Offset.zero & size, paint);

    final unit = (size.height / 24).clamp(3.0, 8.0).toDouble();
    void block(num x, num y, num width, num height, Color color) {
      paint.color = color;
      canvas.drawRect(
        Rect.fromLTWH(
          x.toDouble() * unit,
          y.toDouble() * unit,
          width.toDouble() * unit,
          height.toDouble() * unit,
        ),
        paint,
      );
    }

    // A tiny night-sky map keeps the hero readable while adding game texture.
    block(3, 3, 1, 1, PixelColors.yellow);
    block(8, 2, 2, 1, PixelColors.cream);
    block(17, 4, 1, 1, PixelColors.mint);
    block(24, 2, 1, 1, PixelColors.yellow);
    block(29, 5, 2, 1, PixelColors.cream);
    block(5, 8, 1, 1, PixelColors.sky);
    block(21, 7, 1, 1, PixelColors.sky);
    block(32, 9, 1, 1, PixelColors.mint);

    // Pixel moon and a small floating planet.
    block(2, 11, 4, 1, PixelColors.yellow);
    block(1, 12, 6, 2, PixelColors.yellow);
    block(2, 14, 4, 1, PixelColors.yellow);
    block(27, 11, 3, 1, PixelColors.sky);
    block(26, 12, 5, 2, PixelColors.sky);
    block(27, 14, 3, 1, PixelColors.sky);

    final ox = (size.width / unit).floor() - 16;
    const oy = 3.0;
    final dark = PixelColors.ink;

    // A small pixel mascot rides the cargo pod.
    block(ox + 7, oy - 2, 2, 2, dark);
    block(ox + 6, oy - 3, 4, 1, PixelColors.mint);
    block(ox + 2, oy, 4, 1, dark);
    block(ox + 10, oy, 4, 1, dark);
    block(ox + 1, oy + 1, 6, 4, dark);
    block(ox + 9, oy + 1, 6, 4, dark);
    block(ox + 3, oy + 2, 3, 3, PixelColors.orange);
    block(ox + 10, oy + 2, 3, 3, PixelColors.orange);
    block(ox, oy + 5, 16, 9, dark);
    block(ox + 2, oy + 6, 12, 7, PixelColors.orange);
    block(ox + 4, oy + 8, 8, 4, PixelColors.cream);
    block(ox + 4, oy + 8, 2, 2, dark);
    block(ox + 10, oy + 8, 2, 2, dark);
    block(ox + 5, oy + 8, 1, 1, PixelColors.cream);
    block(ox + 10, oy + 8, 1, 1, PixelColors.cream);
    block(ox + 7, oy + 10, 2, 1, PixelColors.pink);
    block(ox + 6, oy + 11, 1, 1, dark);
    block(ox + 9, oy + 11, 1, 1, dark);
    block(ox + 5, oy + 13, 6, 1, PixelColors.mint);
    block(ox + 2, oy + 14, 12, 1, dark);
    block(ox + 3, oy + 15, 10, 2, PixelColors.sky);
    block(ox + 1, oy + 17, 14, 1, PixelColors.mint);
    block(ox - 3, oy + 18, 4, 1, PixelColors.orange);
    block(ox + 15, oy + 18, 4, 1, PixelColors.orange);

    // Low horizon and simple pixel rails.
    block(0, 21, 40, 1, PixelColors.panelLight);
    block(2, 22, 7, 1, PixelColors.outline);
    block(13, 22, 9, 1, PixelColors.outline);
    block(26, 22, 6, 1, PixelColors.outline);
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}
