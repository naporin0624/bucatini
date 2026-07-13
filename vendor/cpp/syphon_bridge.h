#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef void* SyphonBridgeHandle;

SyphonBridgeHandle syphon_bridge_create(const char* name);
void               syphon_bridge_destroy(SyphonBridgeHandle handle);

// Publish a BGRA buffer. bytes_per_row is the source stride (>= width*4).
// Dispatches to the benchmark-winning send strategy. Returns 0 on success,
// -1 on error.
int syphon_bridge_send_rgba(SyphonBridgeHandle handle,
                            const uint8_t* data,
                            uint32_t width,
                            uint32_t height,
                            uint32_t bytes_per_row);

// Publish a BGRA buffer via a specific send strategy, exposed for benchmarking.
//   0 = per-frame IOSurface + MTLTexture allocation + row memcpy (original)
//   1 = cached IOSurface + MTLTexture reuse + row memcpy
//   2 = mode 1 + single bulk memcpy when source stride == surface row pitch
// Returns 0 on success, -1 on error.
int syphon_bridge_send_rgba_mode(SyphonBridgeHandle handle,
                                 const uint8_t* data,
                                 uint32_t width,
                                 uint32_t height,
                                 uint32_t bytes_per_row,
                                 uint32_t mode);

#ifdef __cplusplus
}
#endif
