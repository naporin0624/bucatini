#import "syphon_bridge.h"
#import <Metal/Metal.h>
#import <IOSurface/IOSurface.h>
#import <Syphon/Syphon.h>
#import <Cocoa/Cocoa.h>

struct SyphonBridge {
    id<MTLDevice>       device;
    id<MTLCommandQueue> commandQueue;
    SyphonMetalServer*  server;
    // Cached surface/texture reused by modes 1 and 2; (re)allocated on resize.
    IOSurfaceRef        surface;      // CFRetained (owned); nil when unallocated
    id<MTLTexture>      texture;      // wraps `surface`, keeping it alive
    uint32_t            width;
    uint32_t            height;
    size_t              bytesPerRow;  // the surface's actual (kernel-aligned) row pitch
};

// Release the cached surface/texture and reset the cached geometry.
static void release_surface(SyphonBridge* b) {
    b->texture = nil;
    if (b->surface) {
        CFRelease(b->surface);
        b->surface = nullptr;
    }
    b->width = 0;
    b->height = 0;
    b->bytesPerRow = 0;
}

// (Re)create the cached IOSurface + MTLTexture when the geometry changed, or when
// the requested source stride no longer fits the surface's row pitch. Returns
// false if either object could not be created.
static bool ensure_surface(SyphonBridge* b, uint32_t width, uint32_t height,
                           uint32_t bytes_per_row) {
    if (b->surface && b->width == width && b->height == height &&
        bytes_per_row <= b->bytesPerRow) {
        return true;
    }
    release_surface(b);

    NSDictionary* surfaceProps = @{
        (NSString*)kIOSurfaceWidth: @(width),
        (NSString*)kIOSurfaceHeight: @(height),
        (NSString*)kIOSurfaceBytesPerElement: @4,
        (NSString*)kIOSurfaceBytesPerRow: @(bytes_per_row),
        (NSString*)kIOSurfacePixelFormat: @(kCVPixelFormatType_32BGRA),
        (NSString*)kIOSurfaceAllocSize: @(bytes_per_row * height)
    };
    IOSurfaceRef surface = IOSurfaceCreate((__bridge CFDictionaryRef)surfaceProps);
    if (!surface) {
        NSLog(@"[SyphonBridge] ERROR: Failed to create IOSurface");
        return false;
    }

    MTLTextureDescriptor* desc =
        [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:MTLPixelFormatBGRA8Unorm
                                                           width:width
                                                          height:height
                                                       mipmapped:NO];
    desc.usage = MTLTextureUsageShaderRead;
    desc.storageMode = MTLStorageModeShared;
    id<MTLTexture> texture = [b->device newTextureWithDescriptor:desc
                                                        iosurface:surface
                                                            plane:0];
    if (!texture) {
        NSLog(@"[SyphonBridge] ERROR: Failed to create Metal texture from IOSurface");
        CFRelease(surface);
        return false;
    }

    b->surface = surface;
    b->texture = texture;
    b->width = width;
    b->height = height;
    b->bytesPerRow = IOSurfaceGetBytesPerRow(surface);
    return true;
}

// Copy row-by-row, honoring both the source stride and the surface's row pitch.
static void copy_rows(IOSurfaceRef surface, const uint8_t* data, uint32_t width,
                      uint32_t height, uint32_t bytes_per_row) {
    IOSurfaceLock(surface, 0, nullptr);
    uint8_t* dstRow = static_cast<uint8_t*>(IOSurfaceGetBaseAddress(surface));
    size_t surfaceBytesPerRow = IOSurfaceGetBytesPerRow(surface);
    const uint8_t* srcRow = data;
    size_t copyWidth = static_cast<size_t>(width) * 4;
    for (uint32_t y = 0; y < height; y++) {
        memcpy(dstRow, srcRow, copyWidth);
        srcRow += bytes_per_row;
        dstRow += surfaceBytesPerRow;
    }
    IOSurfaceUnlock(surface, 0, nullptr);
}

// Copy the whole frame in one memcpy. Valid only when the source stride equals
// the surface's row pitch (checked by the caller).
static void copy_bulk(IOSurfaceRef surface, const uint8_t* data, uint32_t height,
                      uint32_t bytes_per_row) {
    IOSurfaceLock(surface, 0, nullptr);
    void* baseAddr = IOSurfaceGetBaseAddress(surface);
    memcpy(baseAddr, data, static_cast<size_t>(bytes_per_row) * height);
    IOSurfaceUnlock(surface, 0, nullptr);
}

// Commit a Syphon publish of the cached texture on a fresh command buffer.
static int publish_texture(SyphonBridge* b, uint32_t width, uint32_t height) {
    id<MTLCommandBuffer> cmdBuf = [b->commandQueue commandBuffer];
    [b->server publishFrameTexture:b->texture
                   onCommandBuffer:cmdBuf
                       imageRegion:NSMakeRect(0, 0, width, height)
                           flipped:YES];
    [cmdBuf commit];
    return 0;
}

// Mode 0: allocate a fresh IOSurface + MTLTexture every frame, row-copy, publish.
// Kept for benchmark comparison; does not touch the cache.
static int send_perframe_rowcopy(SyphonBridge* b, const uint8_t* data,
                                 uint32_t width, uint32_t height, uint32_t bytes_per_row) {
    NSDictionary* surfaceProps = @{
        (NSString*)kIOSurfaceWidth: @(width),
        (NSString*)kIOSurfaceHeight: @(height),
        (NSString*)kIOSurfaceBytesPerElement: @4,
        (NSString*)kIOSurfaceBytesPerRow: @(bytes_per_row),
        (NSString*)kIOSurfacePixelFormat: @(kCVPixelFormatType_32BGRA),
        (NSString*)kIOSurfaceAllocSize: @(bytes_per_row * height)
    };
    IOSurfaceRef surface = IOSurfaceCreate((__bridge CFDictionaryRef)surfaceProps);
    if (!surface) {
        NSLog(@"[SyphonBridge] ERROR: Failed to create IOSurface");
        return -1;
    }

    copy_rows(surface, data, width, height, bytes_per_row);

    MTLTextureDescriptor* desc =
        [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:MTLPixelFormatBGRA8Unorm
                                                           width:width
                                                          height:height
                                                       mipmapped:NO];
    desc.usage = MTLTextureUsageShaderRead;
    desc.storageMode = MTLStorageModeShared;
    id<MTLTexture> texture = [b->device newTextureWithDescriptor:desc
                                                        iosurface:surface
                                                            plane:0];
    CFRelease(surface);
    if (!texture) {
        NSLog(@"[SyphonBridge] ERROR: Failed to create Metal texture from IOSurface");
        return -1;
    }

    id<MTLCommandBuffer> cmdBuf = [b->commandQueue commandBuffer];
    [b->server publishFrameTexture:texture
                   onCommandBuffer:cmdBuf
                       imageRegion:NSMakeRect(0, 0, width, height)
                           flipped:YES];
    [cmdBuf commit];
    return 0;
}

// Mode 1: reuse the cached surface/texture, row-copy each frame, publish.
static int send_persistent_rowcopy(SyphonBridge* b, const uint8_t* data,
                                   uint32_t width, uint32_t height, uint32_t bytes_per_row) {
    if (!ensure_surface(b, width, height, bytes_per_row)) return -1;
    copy_rows(b->surface, data, width, height, bytes_per_row);
    return publish_texture(b, width, height);
}

// Mode 2: reuse the cached surface/texture; bulk-copy when the source stride
// matches the surface's row pitch, otherwise fall back to a row copy.
static int send_persistent_bulkcopy(SyphonBridge* b, const uint8_t* data,
                                    uint32_t width, uint32_t height, uint32_t bytes_per_row) {
    if (!ensure_surface(b, width, height, bytes_per_row)) return -1;
    if (bytes_per_row == b->bytesPerRow) {
        copy_bulk(b->surface, data, height, bytes_per_row);
    } else {
        copy_rows(b->surface, data, width, height, bytes_per_row);
    }
    return publish_texture(b, width, height);
}

extern "C" {

SyphonBridgeHandle syphon_bridge_create(const char* name) {
    @autoreleasepool {
        auto* bridge = new SyphonBridge();
        bridge->device = MTLCreateSystemDefaultDevice();
        if (!bridge->device) {
            NSLog(@"[SyphonBridge] ERROR: Failed to create Metal device");
            delete bridge;
            return nullptr;
        }
        bridge->commandQueue = [bridge->device newCommandQueue];
        NSString* serverName = [NSString stringWithUTF8String:name];
        bridge->server = [[SyphonMetalServer alloc] initWithName:serverName
                                                          device:bridge->device
                                                         options:nil];
        if (!bridge->server) {
            NSLog(@"[SyphonBridge] ERROR: Failed to create SyphonMetalServer");
            delete bridge;
            return nullptr;
        }
        return static_cast<SyphonBridgeHandle>(bridge);
    }
}

void syphon_bridge_destroy(SyphonBridgeHandle handle) {
    if (!handle) return;
    @autoreleasepool {
        auto* bridge = static_cast<SyphonBridge*>(handle);
        release_surface(bridge);
        [bridge->server stop];
        bridge->server       = nil;
        bridge->commandQueue = nil;
        bridge->device       = nil;
        delete bridge;
    }
}

int syphon_bridge_send_rgba_mode(SyphonBridgeHandle handle,
                                 const uint8_t* data,
                                 uint32_t width,
                                 uint32_t height,
                                 uint32_t bytes_per_row,
                                 uint32_t mode) {
    if (!handle || !data) return -1;
    if (width == 0 || height == 0) return -1;
    @autoreleasepool {
        auto* bridge = static_cast<SyphonBridge*>(handle);
        switch (mode) {
            case 1:  return send_persistent_rowcopy(bridge, data, width, height, bytes_per_row);
            case 2:  return send_persistent_bulkcopy(bridge, data, width, height, bytes_per_row);
            case 0:
            default: return send_perframe_rowcopy(bridge, data, width, height, bytes_per_row);
        }
    }
}

int syphon_bridge_send_rgba(SyphonBridgeHandle handle,
                            const uint8_t* data,
                            uint32_t width,
                            uint32_t height,
                            uint32_t bytes_per_row) {
    // Production default: routed to the benchmark-winning strategy
    // (PersistentRowCopy, mode 1). Reusing one IOSurface+MTLTexture cuts CPU
    // per frame ~4.5x at 1080p and ~3.2x at 4K vs the original per-frame
    // allocation. Bulk copy (mode 2) is within noise since NDI frames arrive
    // tightly packed, so the simpler row copy ships.
    return syphon_bridge_send_rgba_mode(handle, data, width, height, bytes_per_row, 1);
}

} // extern "C"
