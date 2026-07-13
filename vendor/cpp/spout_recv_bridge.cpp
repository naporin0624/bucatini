// spout_recv_bridge.cpp
// C++ bridge for Rust FFI to the Spout2 SDK (receiver, CPU BGRA buffer output).
//
// Companion to spout_bridge.cpp: it opens a sender's shared texture by name and
// reads it back into a CPU BGRA buffer. Used by the color-verification harness
// to prove the send path publishes correct pixels (no R/B swap, no flip).

#include <cstdint>
#include <windows.h>
#include "SpoutDX.h"
#include "spout_recv_bridge.h"

struct SpoutRecv {
    spoutDX receiver;
};

extern "C" {

SpoutRecvHandle spout_recv_create(void) {
    SpoutRecv* recv = new SpoutRecv();
    if (!recv->receiver.OpenDirectX11()) {
        delete recv;
        return nullptr;
    }
    return recv;
}

void spout_recv_destroy(SpoutRecvHandle handle) {
    if (!handle) return;
    SpoutRecv* recv = static_cast<SpoutRecv*>(handle);
    recv->receiver.ReleaseReceiver();
    recv->receiver.CloseDirectX11();
    delete recv;
}

int spout_recv_grab(SpoutRecvHandle handle, const char* name,
                    uint8_t* buf, uint32_t max_w, uint32_t max_h,
                    uint32_t* out_w, uint32_t* out_h, uint32_t timeout_ms) {
    if (!handle || !name || !buf || !out_w || !out_h) return -1;
    SpoutRecv* recv = static_cast<SpoutRecv*>(handle);

    recv->receiver.SetReceiverName(name);

    // SpoutDX::ReceiveImage reads back through a 2-staging ping-pong: on a new
    // frame it copies shared->staging[i] but reads staging[i^1], so the FIRST
    // new-frame read returns the previous (empty) staging. We therefore require
    // two new frames before `buf` is guaranteed to hold real pixels. This needs
    // the sender to be streaming (the caller publishes continuously).
    int new_frames = 0;
    ULONGLONG start = GetTickCount64();
    while (GetTickCount64() - start < timeout_ms) {
        // Always receive at the sender's current size; ReadPixelData copies
        // `width*height`, so passing 0 (before connect) copies nothing -- which
        // is fine because that first call only triggers the connect + update.
        unsigned int w = recv->receiver.GetSenderWidth();
        unsigned int h = recv->receiver.GetSenderHeight();
        recv->receiver.ReceiveImage(buf, w, h, false, false);

        // IsUpdated() both reports and CLEARS the update flag; until it is
        // cleared ReceiveImage short-circuits without copying.
        if (recv->receiver.IsUpdated()) {
            w = recv->receiver.GetSenderWidth();
            h = recv->receiver.GetSenderHeight();
            if (w > max_w || h > max_h) {
                *out_w = w;
                *out_h = h;
                return -2;
            }
            new_frames = 0; // size just (re)learned -- restart the ping-pong count
            continue;
        }

        if (recv->receiver.IsConnected() && w > 0 && h > 0 && recv->receiver.IsFrameNew()) {
            new_frames++;
            if (new_frames >= 2) {
                *out_w = w;
                *out_h = h;
                return 0;
            }
        }
        Sleep(2);
    }

    return -1;
}

} // extern "C"
