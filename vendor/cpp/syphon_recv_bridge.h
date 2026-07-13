#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque handle to a Syphon receiver. Wraps its own Metal device + command queue
// and a cached shared-storage readback texture, so a single process can host both
// a SyphonMetalServer (sender) and this receiver at once.
typedef void* SyphonRecvHandle;

// Create a receiver and its Metal device/queue. Returns NULL on failure.
SyphonRecvHandle syphon_recv_create(void);

// Destroy the receiver and release its GPU resources.
void syphon_recv_destroy(SyphonRecvHandle handle);

// Poll up to `timeout_ms` for one frame from the Syphon server named `name` into
// `buf` (BGRA, capacity max_w*max_h*4 bytes). On success returns 0 and sets
// *out_w/*out_h to the received size (buf filled tightly at out_w*4 bytes/row).
// Returns -2 if the server frame is larger than max_w/max_h (sets *out_w/*out_h
// to the frame size), and -1 on timeout / no server / null argument.
int syphon_recv_grab(SyphonRecvHandle handle, const char* name,
                     uint8_t* buf, uint32_t max_w, uint32_t max_h,
                     uint32_t* out_w, uint32_t* out_h, uint32_t timeout_ms);

#ifdef __cplusplus
}
#endif
