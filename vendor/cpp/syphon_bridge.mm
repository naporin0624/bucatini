#import "syphon_bridge.h"
#import <Metal/Metal.h>
#import <IOSurface/IOSurface.h>
#import <Syphon/Syphon.h>
#import <Cocoa/Cocoa.h>

struct SyphonBridge {
    id<MTLDevice>       device;
    id<MTLCommandQueue> commandQueue;
    SyphonMetalServer*  server;
};

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
        [bridge->server stop];
        bridge->server       = nil;
        bridge->commandQueue = nil;
        bridge->device       = nil;
        delete bridge;
    }
}

int syphon_bridge_send_rgba(SyphonBridgeHandle handle,
                            const uint8_t* data,
                            uint32_t width,
                            uint32_t height,
                            uint32_t bytes_per_row) {
    if (!handle || !data) return -1;
    @autoreleasepool {
        auto* bridge = static_cast<SyphonBridge*>(handle);

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

        IOSurfaceLock(surface, 0, nullptr);
        void* baseAddr = IOSurfaceGetBaseAddress(surface);
        size_t surfaceBytesPerRow = IOSurfaceGetBytesPerRow(surface);
        const uint8_t* srcRow = data;
        uint8_t* dstRow = static_cast<uint8_t*>(baseAddr);
        size_t copyWidth = (size_t)width * 4;
        for (uint32_t y = 0; y < height; y++) {
            memcpy(dstRow, srcRow, copyWidth);
            srcRow += bytes_per_row;
            dstRow += surfaceBytesPerRow;
        }
        IOSurfaceUnlock(surface, 0, nullptr);

        MTLTextureDescriptor* desc =
            [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:MTLPixelFormatBGRA8Unorm
                                                               width:width
                                                              height:height
                                                           mipmapped:NO];
        desc.usage = MTLTextureUsageShaderRead;
        desc.storageMode = MTLStorageModeShared;
        id<MTLTexture> texture = [bridge->device newTextureWithDescriptor:desc
                                                               iosurface:surface
                                                                   plane:0];
        CFRelease(surface);
        if (!texture) {
            NSLog(@"[SyphonBridge] ERROR: Failed to create Metal texture from IOSurface");
            return -1;
        }

        id<MTLCommandBuffer> cmdBuf = [bridge->commandQueue commandBuffer];
        [bridge->server publishFrameTexture:texture
                            onCommandBuffer:cmdBuf
                                imageRegion:NSMakeRect(0, 0, width, height)
                                    flipped:YES];
        [cmdBuf commit];
        return 0;
    }
}

} // extern "C"
