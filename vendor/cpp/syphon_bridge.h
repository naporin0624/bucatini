#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef void* SyphonBridgeHandle;

SyphonBridgeHandle syphon_bridge_create(const char* name);
void               syphon_bridge_destroy(SyphonBridgeHandle handle);

// Publish a BGRA buffer. bytes_per_row is the source stride (>= width*4).
// Returns 0 on success, -1 on error.
int syphon_bridge_send_rgba(SyphonBridgeHandle handle,
                            const uint8_t* data,
                            uint32_t width,
                            uint32_t height,
                            uint32_t bytes_per_row);

#ifdef __cplusplus
}
#endif
