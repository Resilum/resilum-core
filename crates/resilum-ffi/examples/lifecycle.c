// Minimal resilum-ffi usage. Compile against resilum.h and link libresilum_ffi.
#include <stdio.h>
#include "resilum.h"

int main(void) {
    const char *config =
        "instance_name: mobile\n"
        "connect:\n"
        "  services: [socks-egress]\n"
        "  listen_tcp: 127.0.0.1:0\n";

    ResilumNode *node = resilum_node_new_from_yaml(config);
    if (!node) {
        fprintf(stderr, "config error: %s\n", resilum_last_error());
        return 1;
    }
    if (resilum_node_start(node) != RESILUM_OK) {
        fprintf(stderr, "start error: %s\n", resilum_last_error());
        resilum_node_free(node);
        return 1;
    }

    // Point the app's traffic at this local SOCKS port.
    printf("SOCKS on 127.0.0.1:%u\n", resilum_node_socks_port(node));

    // Drain status events.
    ResilumEvent *event;
    while ((event = resilum_node_poll_event(node)) != NULL) {
        printf("event kind=%d\n", resilum_event_kind(event));
        resilum_event_free(event);
    }

    resilum_node_stop(node);
    resilum_node_free(node);
    return 0;
}
