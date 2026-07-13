#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque handle to a Spout sender.
typedef void* SpoutBridgeHandle;

// Create a Spout sender publishing under `name`. The DirectX 11 device and the
// staging texture are created lazily; the texture is (re)allocated on the first
// frame and whenever the dimensions change.
// Returns NULL on failure.
SpoutBridgeHandle spout_bridge_create(const char* name);

// Destroy the sender and release all GPU resources.
void spout_bridge_destroy(SpoutBridgeHandle handle);

// Publish one BGRA frame. `data` is a BGRA byte buffer, `bytes_per_row` is the
// source stride (>= width*4; NDI frames may include padding). Internally the
// rows are copied into a DYNAMIC B8G8R8A8_UNORM texture (honoring both the
// source stride and the mapped row pitch) and sent via SpoutDX::SendTexture.
// Returns 0 on success, negative on error.
int spout_bridge_send_rgba(SpoutBridgeHandle handle,
                           const uint8_t* data,
                           uint32_t width,
                           uint32_t height,
                           uint32_t bytes_per_row);

// Send strategies, exposed for benchmarking. spout_bridge_send_rgba() dispatches
// to the strategy chosen as the production winner.
//   0 = DYNAMIC staging + row-by-row memcpy + SendTexture   (original)
//   1 = SendImage: UpdateSubresource straight into the shared texture (no staging)
//   2 = DYNAMIC staging + SpoutCopy SSE2 (pitch-aware) + SendTexture
int spout_bridge_send_rgba_mode(SpoutBridgeHandle handle,
                                const uint8_t* data, uint32_t width, uint32_t height,
                                uint32_t bytes_per_row, uint32_t mode);

#ifdef __cplusplus
}
#endif
