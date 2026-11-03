// Rebuild from this directory:
// clang --target=armv7-linux-gnueabihf -fuse-ld=lld -shared -nostdlib \
//   -Wl,--build-id=none -Wl,-soname,linux-arm.node linux-arm.c -o linux-arm.node
// Format-only fixture; never load or execute it.
int nrz_foreign_fixture(void) { return 0; }
