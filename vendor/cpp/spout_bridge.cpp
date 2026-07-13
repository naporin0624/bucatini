// spout_bridge.cpp
// C++ bridge for Rust FFI to the Spout2 SDK (sender, CPU BGRA buffer input).
//
// Mirrors the macOS Syphon sender shim: it takes a BGRA byte buffer + stride
// and publishes it as a shared GPU texture. Unlike electron-texture-bridge's
// Spout sender (which receives an already-shared DXGI texture handle from
// Electron), this path uploads a CPU buffer from NDI into a DYNAMIC D3D11
// texture and sends that via SpoutDX::SendTexture.

#include <cstdint>
#include <cstring>
#include <string>
#include <d3d11.h>
#include "SpoutDX.h"
#include "SpoutCopy.h"
#include "spout_bridge.h"

struct SpoutBridge {
    spoutDX sender;
    spoutCopy copier;                    // SSE2/pitch-aware line copier (strategy 2)
    ID3D11Device* device = nullptr;
    ID3D11DeviceContext* context = nullptr;
    ID3D11Texture2D* staging = nullptr;  // DYNAMIC upload texture, (re)allocated on resize
    unsigned int width = 0;
    unsigned int height = 0;
    bool initialized = false;
};

static void release_staging(SpoutBridge* b) {
    if (b->staging) {
        b->staging->Release();
        b->staging = nullptr;
    }
    b->width = 0;
    b->height = 0;
}

// (Re)allocate the DYNAMIC staging texture when the size changes. Returns false
// if the texture could not be created.
static bool ensure_staging(SpoutBridge* b, uint32_t width, uint32_t height) {
    if (b->staging && b->width == width && b->height == height) {
        return true;
    }
    release_staging(b);

    D3D11_TEXTURE2D_DESC desc = {};
    desc.Width = width;
    desc.Height = height;
    desc.MipLevels = 1;
    desc.ArraySize = 1;
    desc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
    desc.SampleDesc.Count = 1;
    desc.SampleDesc.Quality = 0;
    desc.Usage = D3D11_USAGE_DYNAMIC;
    desc.BindFlags = D3D11_BIND_SHADER_RESOURCE;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE;

    HRESULT hr = b->device->CreateTexture2D(&desc, nullptr, &b->staging);
    if (FAILED(hr) || !b->staging) {
        release_staging(b);
        return false;
    }
    b->width = width;
    b->height = height;
    return true;
}

// Strategy 0: DYNAMIC staging + row-by-row memcpy + SendTexture (original path).
static int send_staging_rowcopy(SpoutBridge* b, const uint8_t* data,
                                uint32_t width, uint32_t height, uint32_t bytes_per_row) {
    if (!ensure_staging(b, width, height)) return -3;

    D3D11_MAPPED_SUBRESOURCE mapped;
    HRESULT hr = b->context->Map(b->staging, 0, D3D11_MAP_WRITE_DISCARD, 0, &mapped);
    if (FAILED(hr)) return -4;

    // Honor both the source stride and the mapped row pitch (they may differ).
    const uint8_t* srcRow = data;
    uint8_t* dstRow = static_cast<uint8_t*>(mapped.pData);
    const size_t copyWidth = static_cast<size_t>(width) * 4;
    for (uint32_t y = 0; y < height; ++y) {
        memcpy(dstRow, srcRow, copyWidth);
        srcRow += bytes_per_row;
        dstRow += mapped.RowPitch;
    }
    b->context->Unmap(b->staging, 0);

    return b->sender.SendTexture(b->staging) ? 0 : -5;
}

// Strategy 1: SendImage writes the CPU buffer directly into the shared texture
// via UpdateSubresource -- no staging texture and no CopyResource.
static int send_sendimage(SpoutBridge* b, const uint8_t* data,
                          uint32_t width, uint32_t height, uint32_t bytes_per_row) {
    return b->sender.SendImage(data, width, height, bytes_per_row) ? 0 : -5;
}

// Strategy 2: DYNAMIC staging, but fill it with SpoutCopy's SSE2 line copier
// (pitch-aware) instead of a scalar memcpy loop, then SendTexture.
static int send_staging_sse(SpoutBridge* b, const uint8_t* data,
                            uint32_t width, uint32_t height, uint32_t bytes_per_row) {
    if (!ensure_staging(b, width, height)) return -3;

    D3D11_MAPPED_SUBRESOURCE mapped;
    HRESULT hr = b->context->Map(b->staging, 0, D3D11_MAP_WRITE_DISCARD, 0, &mapped);
    if (FAILED(hr)) return -4;

    b->copier.rgba2rgba(data, mapped.pData, width, height, bytes_per_row, mapped.RowPitch, false);
    b->context->Unmap(b->staging, 0);

    return b->sender.SendTexture(b->staging) ? 0 : -5;
}

extern "C" {

SpoutBridgeHandle spout_bridge_create(const char* name) {
    SpoutBridge* bridge = new SpoutBridge();

    if (!bridge->sender.OpenDirectX11()) {
        delete bridge;
        return nullptr;
    }
    bridge->device = bridge->sender.GetDX11Device();
    bridge->context = bridge->sender.GetDX11Context();
    if (!bridge->device || !bridge->context) {
        bridge->sender.CloseDirectX11();
        delete bridge;
        return nullptr;
    }

    if (!bridge->sender.SetSenderName(name)) {
        bridge->sender.CloseDirectX11();
        delete bridge;
        return nullptr;
    }

    // Publish as BGRA so NDI's BGRA frames need no channel swizzle.
    bridge->sender.SetSenderFormat(DXGI_FORMAT_B8G8R8A8_UNORM);

    bridge->initialized = true;
    return bridge;
}

void spout_bridge_destroy(SpoutBridgeHandle handle) {
    if (!handle) return;
    SpoutBridge* bridge = static_cast<SpoutBridge*>(handle);
    release_staging(bridge);
    bridge->sender.ReleaseSender();
    bridge->sender.CloseDirectX11();
    delete bridge;
}

int spout_bridge_send_rgba_mode(SpoutBridgeHandle handle,
                                const uint8_t* data, uint32_t width, uint32_t height,
                                uint32_t bytes_per_row, uint32_t mode) {
    if (!handle || !data) return -1;
    SpoutBridge* bridge = static_cast<SpoutBridge*>(handle);
    if (!bridge->initialized || !bridge->device || !bridge->context) return -2;
    if (width == 0 || height == 0) return -1;

    switch (mode) {
        case 1:  return send_sendimage(bridge, data, width, height, bytes_per_row);
        case 2:  return send_staging_sse(bridge, data, width, height, bytes_per_row);
        case 0:
        default: return send_staging_rowcopy(bridge, data, width, height, bytes_per_row);
    }
}

int spout_bridge_send_rgba(SpoutBridgeHandle handle,
                           const uint8_t* data,
                           uint32_t width,
                           uint32_t height,
                           uint32_t bytes_per_row) {
    // Production default: routed to the benchmark-winning strategy (StagingSse,
    // mode 2). It wins on CPU time at 4K -- where sustaining 60fps is hardest --
    // and is within noise of the original row-copy at 1080p. SendImage's
    // UpdateSubresource path was ~3x worse (synchronous copy on this thread).
    return spout_bridge_send_rgba_mode(handle, data, width, height, bytes_per_row, 2);
}

} // extern "C"
