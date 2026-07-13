#import "syphon_recv_bridge.h"
#import <Metal/Metal.h>
#import <Syphon/Syphon.h>
#import <Syphon/SyphonServerDirectory.h>
#import <Syphon/SyphonMetalClient.h>
#import <CoreFoundation/CoreFoundation.h>

struct SyphonRecv {
    id<MTLDevice>       device;
    id<MTLCommandQueue> commandQueue;
    // Shared-storage texture that a blit copies each frame into so getBytes can
    // read it on the CPU; (re)allocated when the frame geometry changes.
    id<MTLTexture>      readback;
    uint32_t            rbWidth;
    uint32_t            rbHeight;
};

// (Re)create the cached shared-storage readback texture when the frame geometry
// changed. Returns false if the texture could not be created.
static bool ensure_readback(SyphonRecv* r, uint32_t width, uint32_t height) {
    if (r->readback && r->rbWidth == width && r->rbHeight == height) {
        return true;
    }
    MTLTextureDescriptor* desc =
        [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:MTLPixelFormatBGRA8Unorm
                                                           width:width
                                                          height:height
                                                       mipmapped:NO];
    desc.usage = MTLTextureUsageShaderRead;
    desc.storageMode = MTLStorageModeShared;
    id<MTLTexture> texture = [r->device newTextureWithDescriptor:desc];
    if (!texture) {
        NSLog(@"[SyphonRecv] ERROR: Failed to create readback texture");
        return false;
    }
    r->readback = texture;
    r->rbWidth = width;
    r->rbHeight = height;
    return true;
}

// Blit the frame texture into the cached readback texture and copy it out to
// `buf` as tightly packed BGRA (stride = width*4). The frame is a live view of
// the server's IOSurface, so a completed blit gives a stable CPU snapshot.
static void read_frame_into(SyphonRecv* r, id<MTLTexture> frame,
                            uint32_t width, uint32_t height, uint8_t* buf) {
    id<MTLCommandBuffer> cmdBuf = [r->commandQueue commandBuffer];
    id<MTLBlitCommandEncoder> blit = [cmdBuf blitCommandEncoder];
    [blit copyFromTexture:frame
              sourceSlice:0
              sourceLevel:0
             sourceOrigin:MTLOriginMake(0, 0, 0)
               sourceSize:MTLSizeMake(width, height, 1)
                toTexture:r->readback
         destinationSlice:0
         destinationLevel:0
        destinationOrigin:MTLOriginMake(0, 0, 0)];
    [blit endEncoding];
    [cmdBuf commit];
    [cmdBuf waitUntilCompleted];

    MTLRegion region = MTLRegionMake2D(0, 0, width, height);
    [r->readback getBytes:buf
              bytesPerRow:(NSUInteger)width * 4
               fromRegion:region
              mipmapLevel:0];
}

extern "C" {

SyphonRecvHandle syphon_recv_create(void) {
    @autoreleasepool {
        auto* recv = new SyphonRecv();
        recv->device = MTLCreateSystemDefaultDevice();
        if (!recv->device) {
            NSLog(@"[SyphonRecv] ERROR: Failed to create Metal device");
            delete recv;
            return nullptr;
        }
        recv->commandQueue = [recv->device newCommandQueue];
        if (!recv->commandQueue) {
            NSLog(@"[SyphonRecv] ERROR: Failed to create command queue");
            recv->device = nil;
            delete recv;
            return nullptr;
        }
        recv->readback = nil;
        recv->rbWidth = 0;
        recv->rbHeight = 0;
        return static_cast<SyphonRecvHandle>(recv);
    }
}

void syphon_recv_destroy(SyphonRecvHandle handle) {
    if (!handle) return;
    @autoreleasepool {
        auto* recv = static_cast<SyphonRecv*>(handle);
        recv->readback     = nil;
        recv->commandQueue = nil;
        recv->device       = nil;
        delete recv;
    }
}

int syphon_recv_grab(SyphonRecvHandle handle, const char* name,
                     uint8_t* buf, uint32_t max_w, uint32_t max_h,
                     uint32_t* out_w, uint32_t* out_h, uint32_t timeout_ms) {
    if (!handle || !name || !buf || !out_w || !out_h) return -1;
    auto* recv = static_cast<SyphonRecv*>(handle);

    @autoreleasepool {
        NSString* target = [NSString stringWithUTF8String:name];
        SyphonServerDirectory* dir = [SyphonServerDirectory sharedDirectory];
        SyphonMetalClient* client = nil;

        CFAbsoluteTime start = CFAbsoluteTimeGetCurrent();
        double timeoutSec = (double)timeout_ms / 1000.0;

        while (CFAbsoluteTimeGetCurrent() - start < timeoutSec) {
            @autoreleasepool {
                // SyphonServerDirectory is populated by notifications delivered on
                // the run loop, so pump it before querying -- without this the
                // directory stays empty even for an in-process server.
                CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.02, false);

                if (!client) {
                    NSArray* matches = [dir serversMatchingName:target appName:nil];
                    if (matches.count == 0) {
                        continue;
                    }
                    client = [[SyphonMetalClient alloc] initWithServerDescription:matches[0]
                                                                          device:recv->device
                                                                         options:nil
                                                                 newFrameHandler:nil];
                    if (!client) {
                        continue;
                    }
                }

                if (![client isValid]) {
                    continue;
                }

                id<MTLTexture> frame = [client newFrameImage];
                if (!frame) {
                    continue;
                }
                uint32_t w = (uint32_t)frame.width;
                uint32_t h = (uint32_t)frame.height;
                if (w == 0 || h == 0) {
                    continue;
                }
                if (w > max_w || h > max_h) {
                    *out_w = w;
                    *out_h = h;
                    [client stop];
                    return -2;
                }
                if (!ensure_readback(recv, w, h)) {
                    [client stop];
                    return -1;
                }

                read_frame_into(recv, frame, w, h, buf);
                *out_w = w;
                *out_h = h;
                [client stop];
                return 0;
            }
        }

        if (client) {
            [client stop];
        }
        return -1;
    }
}

} // extern "C"
