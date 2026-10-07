import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:onreza_native_fixture/native_fixture.dart';

Future<void> main(List<String> arguments) async {
  if (answer() != 42) throw StateError('native hook returned the wrong result');
  final message = await File('assets/message.txt').readAsString();
  if (Platform.environment['NRZ_FIXTURE_NATIVE_LIBRARY'] == '1') {
    final library = DynamicLibrary.open('lib/libfixture.so');
    final answer = library.lookupFunction<Int32 Function(), int Function()>(
      'onreza_fixture_answer',
    );
    if (answer() != 42)
      throw StateError('native library returned the wrong result');
  }
  if (arguments.isNotEmpty && arguments.first == '--self-check') {
    stdout.writeln(
      jsonEncode({'message': message, 'args': arguments.skip(1).toList()}),
    );
    return;
  }
  final server = await HttpServer.bind(
    InternetAddress.anyIPv4,
    int.parse(Platform.environment['PORT']!),
  );
  ProcessSignal.sigterm.watch().listen((_) async {
    await server.close(force: true);
    exit(0);
  });
  await for (final request in server) {
    request.response.write(switch (request.uri.path) {
      '/ready' => 'ready',
      '/argv' => jsonEncode(arguments),
      _ => message,
    });
    await request.response.close();
  }
}
