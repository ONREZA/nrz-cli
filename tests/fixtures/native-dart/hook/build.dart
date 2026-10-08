import 'dart:io';

import 'package:hooks/hooks.dart';
import 'package:code_assets/code_assets.dart';

void main(List<String> args) async {
  await build(args, (input, output) async {
    if (!input.config.buildCodeAssets) return;
    final library = input.outputDirectory.resolve('libfixture.so');
    final result = await Process.run(
      Platform.environment['NRZ_CC_BIN'] ?? 'cc',
      [
        '-shared',
        '-fPIC',
        '-o',
        library.toFilePath(),
        input.packageRoot.resolve('libfixture.c').toFilePath(),
      ],
    );
    if (result.exitCode != 0) throw StateError('${result.stderr}');
    output.assets.code.add(
      CodeAsset(
        package: input.packageName,
        name: 'native_fixture.dart',
        file: library,
        linkMode: DynamicLoadingBundled(),
      ),
    );
  });
}
