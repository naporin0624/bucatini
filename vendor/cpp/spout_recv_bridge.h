#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque handle to a Spout receiver. Wraps its own spoutDX instance with a
// private DirectX 11 device, so a single process can host both a sender and a
// receiver at once (each OpenDirectX11 creates an independent device).
typedef void* SpoutRecvHandle;

// Create a receiver and open its DirectX 11 device. Returns NULL on failure.
SpoutRecvHandle spout_recv_create(void);

// Destroy the receiver and release its GPU resources.
void spout_recv_destroy(SpoutRecvHandle handle);

// Poll up to `timeout_ms` for one frame from sender `name` into `buf` (BGRA,
// capacity max_w*max_h*4 bytes). On success returns 0 and sets *out_w/*out_h to
// the received size (buf filled tightly at out_w*4 bytes/row). Returns -2 if the
// sender is larger than max_w/max_h (sets *out_w/*out_h to the sender size), and
// -1 on timeout / no sender / null argument.
int spout_recv_grab(SpoutRecvHandle handle, const char* name,
                    uint8_t* buf, uint32_t max_w, uint32_t max_h,
                    uint32_t* out_w, uint32_t* out_h, uint32_t timeout_ms);

#ifdef __cplusplus
}
#endif
