import 'dart:ffi';

@Native<Int32 Function()>(symbol: 'onreza_fixture_answer')
external int answer();
