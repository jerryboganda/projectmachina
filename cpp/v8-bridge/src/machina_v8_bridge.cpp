#include "machina_v8_bridge.h"

#include <new>

struct MachinaV8Bridge {
    uint32_t abi_version;
};

const char *machina_v8_bridge_abi_version(void) {
    return "0.1.0-alpha";
}

MachinaV8Bridge *machina_v8_bridge_create(void) {
    return new (std::nothrow) MachinaV8Bridge{1U};
}

void machina_v8_bridge_destroy(MachinaV8Bridge *bridge) {
    delete bridge;
}
