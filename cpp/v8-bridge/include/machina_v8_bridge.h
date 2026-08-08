#ifndef MACHINA_V8_BRIDGE_H
#define MACHINA_V8_BRIDGE_H

#include <stdint.h>

#if defined(_WIN32)
#define MACHINA_V8_BRIDGE_EXPORT __declspec(dllexport)
#else
#define MACHINA_V8_BRIDGE_EXPORT __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C" {
#endif

/*
 * The Rust facade owns the opaque handle returned by create and must release
 * it exactly once with destroy. The bridge does not retain caller pointers.
 * This bootstrap ABI has no V8 isolate yet; M2-T06 owns the audited V8
 * ownership boundary and will preserve this explicit lifetime contract.
 */
typedef struct MachinaV8Bridge MachinaV8Bridge;

MACHINA_V8_BRIDGE_EXPORT const char *machina_v8_bridge_abi_version(void);
MACHINA_V8_BRIDGE_EXPORT MachinaV8Bridge *machina_v8_bridge_create(void);
MACHINA_V8_BRIDGE_EXPORT void machina_v8_bridge_destroy(MachinaV8Bridge *bridge);

#ifdef __cplusplus
}
#endif

#endif
