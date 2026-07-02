# Changelog

## [0.4.1](https://github.com/naporin0624/bucatini/compare/bucatini-v0.4.0...bucatini-v0.4.1) (2026-07-02)


### Bug Fixes

* **ci:** pass absolute paths to ISCC for the Windows installer ([dc668b9](https://github.com/naporin0624/bucatini/commit/dc668b92b8a7d6a3036f680236e164c3ba14addc))
* **ci:** pass absolute paths to ISCC for the Windows installer ([e508952](https://github.com/naporin0624/bucatini/commit/e508952d48e219ec4cb35e9fd94cdb64388e0fdc))

## [0.4.0](https://github.com/naporin0624/bucatini/compare/bucatini-v0.3.0...bucatini-v0.4.0) (2026-06-30)


### Features

* **gui:** add group_thousands helper with thousands separators ([8ab2a1f](https://github.com/naporin0624/bucatini/commit/8ab2a1fdb28be95b9dfdb90835be2df4ec1ab5a8))
* **gui:** contextual status block + WCAG AA color tokens ([d7d6a35](https://github.com/naporin0624/bucatini/commit/d7d6a35969d02654efe9860ac58188883ecc6e1e))
* **gui:** 文脈的ステータス表示 + WCAG 2.1 AA カラートークン ([2c601c4](https://github.com/naporin0624/bucatini/commit/2c601c4b8f4e033726d4c666df434efdb289ae2f))

## [0.3.0](https://github.com/naporin0624/bucatini/compare/bucatini-v0.2.2...bucatini-v0.3.0) (2026-06-28)


### Features

* add CLI args and source-name matching ([9e2c39f](https://github.com/naporin0624/bucatini/commit/9e2c39f0d8f34186a9cd9260b4a14782bd6d3dea))
* add hand-rolled NDI FFI bindings ([b8e1746](https://github.com/naporin0624/bucatini/commit/b8e1746eb65adecb3d70b96a1bf1af8939061def))
* add interactive selection parsing ([3460a38](https://github.com/naporin0624/bucatini/commit/3460a3894f32f0a2b8a312ca7ed02c56c6ce2ac1))
* add NDI receiver and frame capture ([87e4087](https://github.com/naporin0624/bucatini/commit/87e40874fd311636b828885bab06e8a74337beb6))
* add safe NDI discovery wrapper ([ea15d3c](https://github.com/naporin0624/bucatini/commit/ea15d3c8633f3cfab8d27c69444a537b3bdcdb76))
* add SharedTextureOutput trait and BgraFrame ([21c9de1](https://github.com/naporin0624/bucatini/commit/21c9de1ac7ec1b5f48c853d65a42a7433a4db425))
* **gui:** add tray icon with status + Quit ([a6abb2a](https://github.com/naporin0624/bucatini/commit/a6abb2a1c4d2c1356ec63c28ee4d22df486b93a3))
* **gui:** bundle LINE Seed JP + apply cannelloni dark theme ([5825426](https://github.com/naporin0624/bucatini/commit/5825426768af1448cd49fab85e99e658e72c244c))
* **gui:** close-to-tray with background republishing + restore ([706dae7](https://github.com/naporin0624/bucatini/commit/706dae7b2ae7577086c229aedc2a41a614c1e64a))
* **gui:** discover NDI sources off-thread with source/name form ([9ab36e7](https://github.com/naporin0624/bucatini/commit/9ab36e7ecc27363bbee3289067dd7a49f7abb0f8))
* **gui:** minimal egui launcher (ndi-share-gui) ([dd7133f](https://github.com/naporin0624/bucatini/commit/dd7133fdae371c1121398529bdc30f1d4f04fce1))
* **gui:** quit on Cmd/Ctrl+Q (window ✕ still hides to tray) ([f4759fb](https://github.com/naporin0624/bucatini/commit/f4759fbb279dac081180efc8b64b8e07a04d92d8))
* **gui:** scaffold ndi-share-gui behind the gui feature ([26f9595](https://github.com/naporin0624/bucatini/commit/26f9595db36fe23ce24cb204aaf21d88e73ebce9))
* **gui:** slim layout — drop Name field, single info line ([a38ed23](https://github.com/naporin0624/bucatini/commit/a38ed236aba8d3f4d6d4f36ed1a596108ab816d0))
* **gui:** start/stop the receive loop with live frame-count status ([c9729ee](https://github.com/naporin0624/bucatini/commit/c9729ee9d331939f5d42b4e5ddf94e2f12d2101c))
* implement SyphonOutput backend ([f253ebd](https://github.com/naporin0624/bucatini/commit/f253ebd683100dead8f53b90301724d7cbab0b69))
* rename to bucatini and ship macOS/Windows installers ([e694034](https://github.com/naporin0624/bucatini/commit/e69403416591a65cc68e28a9427a9752c5a25d67))
* rename to bucatini and ship macOS/Windows installers ([d62a74c](https://github.com/naporin0624/bucatini/commit/d62a74ce6492b4072863fa2350551997b763fa77))
* **run:** add frame bounds check and per-frame publish helper ([fe76bd9](https://github.com/naporin0624/bucatini/commit/fe76bd98a3f2e20cfcfe744a5e5c62f4e39e8432))
* **run:** add FrameStream trait and testable run_capture_loop ([b0d19c4](https://github.com/naporin0624/bucatini/commit/b0d19c48c3216d5cefdd987ea70cc58da0adc9b5))
* **windows:** add Spout output backend (compile-verified via CI) ([15ceb91](https://github.com/naporin0624/bucatini/commit/15ceb91e703cf7f5984e34501cec6de294f4c607))
* wire NDI discovery, receive, and Syphon publish loop ([3f3f6b0](https://github.com/naporin0624/bucatini/commit/3f3f6b042384cee03b9927ea819f1627b2e218f1))


### Bug Fixes

* guard against malformed NDI frames and broaden libndi search path ([73f5ed2](https://github.com/naporin0624/bucatini/commit/73f5ed22774fa68f79d47d5ce7650c9b962cea5e))
* **gui:** cap source ComboBox width + truncate so long names don't widen the window ([6e5d308](https://github.com/naporin0624/bucatini/commit/6e5d308aa92d0ea538d850b5425812f2f1403bfb))
* **gui:** harden fetch-fonts.sh (find -print -quit, anchored gitignore) ([efba0c2](https://github.com/naporin0624/bucatini/commit/efba0c2b6a03e3c0c11b9429f31c8a571995e05b))
* **gui:** let tray Quit through the close interceptor (quitting flag) ([41a7872](https://github.com/naporin0624/bucatini/commit/41a78729321dd8816e9bf17eca783f9b894380fa))
* **gui:** newline-separated info + auto-fit window height (no idle gap) ([82136d7](https://github.com/naporin0624/bucatini/commit/82136d75f2b44098ec5f7bcb8e99125df4ed2ae0))
* **gui:** pin refresh icon right via right_to_left, dropdown fills the rest ([0f4bf64](https://github.com/naporin0624/bucatini/commit/0f4bf64edc9f2de0018c500fa898b97cf10596f9))
* **gui:** reserve icon slot so dropdown+refresh always fit the window width ([f888df1](https://github.com/naporin0624/bucatini/commit/f888df1c2a17315bacb63e4c9fb06edb1242f508))
* **gui:** surface unexpected discovery-thread exit instead of hanging ([1a4e0e0](https://github.com/naporin0624/bucatini/commit/1a4e0e01a074a8f1d450a2435dfb88a022dbcd50))
* **gui:** widen window + trim combo so the refresh icon isn't clipped ([b3a34c7](https://github.com/naporin0624/bucatini/commit/b3a34c7851846694326960d74746b46ee648b664))
* **gui:** wrap info line + always-on status row to prevent layout shift ([c6dd801](https://github.com/naporin0624/bucatini/commit/c6dd8018550e495db5509d2d3fb5876c3519fe52))
* harden VideoFrame::data and silence dead_code warnings ([90e829f](https://github.com/naporin0624/bucatini/commit/90e829f7ef1e130e8c135890dfbdbc0e92cc87dd))
* **release:** build a universal macOS binary instead of per-arch on Intel runners ([c2ecec7](https://github.com/naporin0624/bucatini/commit/c2ecec72e39ee0b1fb08485d385264a9b1659ac1))
* **release:** universal macOS binary (no Intel runner dependency) ([91e1641](https://github.com/naporin0624/bucatini/commit/91e1641ed5abfaaca0032110fb639e3cfecbff42))
* **windows:** delay-load NDI DLL + Windows release CD ([ede6fc3](https://github.com/naporin0624/bucatini/commit/ede6fc36ba175c69d6be582ba960608b076c116c))
* **windows:** delay-load NDI DLL and resolve it from the runtime dir ([d5aed96](https://github.com/naporin0624/bucatini/commit/d5aed96c8f1dd7fbd1b505d1060325f8dc41886d))

## [0.2.2](https://github.com/naporin0624/ndi-to-sharedtexture/compare/ndi-share-v0.2.1...ndi-share-v0.2.2) (2026-06-28)


### Bug Fixes

* **windows:** delay-load NDI DLL + Windows release CD ([ede6fc3](https://github.com/naporin0624/ndi-to-sharedtexture/commit/ede6fc36ba175c69d6be582ba960608b076c116c))
* **windows:** delay-load NDI DLL and resolve it from the runtime dir ([d5aed96](https://github.com/naporin0624/ndi-to-sharedtexture/commit/d5aed96c8f1dd7fbd1b505d1060325f8dc41886d))

## [0.2.1](https://github.com/naporin0624/ndi-to-sharedtexture/compare/ndi-share-v0.2.0...ndi-share-v0.2.1) (2026-06-28)


### Bug Fixes

* **release:** build a universal macOS binary instead of per-arch on Intel runners ([c2ecec7](https://github.com/naporin0624/ndi-to-sharedtexture/commit/c2ecec72e39ee0b1fb08485d385264a9b1659ac1))
* **release:** universal macOS binary (no Intel runner dependency) ([91e1641](https://github.com/naporin0624/ndi-to-sharedtexture/commit/91e1641ed5abfaaca0032110fb639e3cfecbff42))

## [0.2.0](https://github.com/naporin0624/ndi-to-sharedtexture/compare/ndi-share-v0.1.0...ndi-share-v0.2.0) (2026-06-28)


### Features

* add CLI args and source-name matching ([9e2c39f](https://github.com/naporin0624/ndi-to-sharedtexture/commit/9e2c39f0d8f34186a9cd9260b4a14782bd6d3dea))
* add hand-rolled NDI FFI bindings ([b8e1746](https://github.com/naporin0624/ndi-to-sharedtexture/commit/b8e1746eb65adecb3d70b96a1bf1af8939061def))
* add interactive selection parsing ([3460a38](https://github.com/naporin0624/ndi-to-sharedtexture/commit/3460a3894f32f0a2b8a312ca7ed02c56c6ce2ac1))
* add NDI receiver and frame capture ([87e4087](https://github.com/naporin0624/ndi-to-sharedtexture/commit/87e40874fd311636b828885bab06e8a74337beb6))
* add safe NDI discovery wrapper ([ea15d3c](https://github.com/naporin0624/ndi-to-sharedtexture/commit/ea15d3c8633f3cfab8d27c69444a537b3bdcdb76))
* add SharedTextureOutput trait and BgraFrame ([21c9de1](https://github.com/naporin0624/ndi-to-sharedtexture/commit/21c9de1ac7ec1b5f48c853d65a42a7433a4db425))
* **gui:** add tray icon with status + Quit ([a6abb2a](https://github.com/naporin0624/ndi-to-sharedtexture/commit/a6abb2a1c4d2c1356ec63c28ee4d22df486b93a3))
* **gui:** bundle LINE Seed JP + apply cannelloni dark theme ([5825426](https://github.com/naporin0624/ndi-to-sharedtexture/commit/5825426768af1448cd49fab85e99e658e72c244c))
* **gui:** close-to-tray with background republishing + restore ([706dae7](https://github.com/naporin0624/ndi-to-sharedtexture/commit/706dae7b2ae7577086c229aedc2a41a614c1e64a))
* **gui:** discover NDI sources off-thread with source/name form ([9ab36e7](https://github.com/naporin0624/ndi-to-sharedtexture/commit/9ab36e7ecc27363bbee3289067dd7a49f7abb0f8))
* **gui:** minimal egui launcher (ndi-share-gui) ([dd7133f](https://github.com/naporin0624/ndi-to-sharedtexture/commit/dd7133fdae371c1121398529bdc30f1d4f04fce1))
* **gui:** quit on Cmd/Ctrl+Q (window ✕ still hides to tray) ([f4759fb](https://github.com/naporin0624/ndi-to-sharedtexture/commit/f4759fbb279dac081180efc8b64b8e07a04d92d8))
* **gui:** scaffold ndi-share-gui behind the gui feature ([26f9595](https://github.com/naporin0624/ndi-to-sharedtexture/commit/26f9595db36fe23ce24cb204aaf21d88e73ebce9))
* **gui:** slim layout — drop Name field, single info line ([a38ed23](https://github.com/naporin0624/ndi-to-sharedtexture/commit/a38ed236aba8d3f4d6d4f36ed1a596108ab816d0))
* **gui:** start/stop the receive loop with live frame-count status ([c9729ee](https://github.com/naporin0624/ndi-to-sharedtexture/commit/c9729ee9d331939f5d42b4e5ddf94e2f12d2101c))
* implement SyphonOutput backend ([f253ebd](https://github.com/naporin0624/ndi-to-sharedtexture/commit/f253ebd683100dead8f53b90301724d7cbab0b69))
* **run:** add frame bounds check and per-frame publish helper ([fe76bd9](https://github.com/naporin0624/ndi-to-sharedtexture/commit/fe76bd98a3f2e20cfcfe744a5e5c62f4e39e8432))
* **run:** add FrameStream trait and testable run_capture_loop ([b0d19c4](https://github.com/naporin0624/ndi-to-sharedtexture/commit/b0d19c48c3216d5cefdd987ea70cc58da0adc9b5))
* **windows:** add Spout output backend (compile-verified via CI) ([15ceb91](https://github.com/naporin0624/ndi-to-sharedtexture/commit/15ceb91e703cf7f5984e34501cec6de294f4c607))
* wire NDI discovery, receive, and Syphon publish loop ([3f3f6b0](https://github.com/naporin0624/ndi-to-sharedtexture/commit/3f3f6b042384cee03b9927ea819f1627b2e218f1))


### Bug Fixes

* guard against malformed NDI frames and broaden libndi search path ([73f5ed2](https://github.com/naporin0624/ndi-to-sharedtexture/commit/73f5ed22774fa68f79d47d5ce7650c9b962cea5e))
* **gui:** cap source ComboBox width + truncate so long names don't widen the window ([6e5d308](https://github.com/naporin0624/ndi-to-sharedtexture/commit/6e5d308aa92d0ea538d850b5425812f2f1403bfb))
* **gui:** harden fetch-fonts.sh (find -print -quit, anchored gitignore) ([efba0c2](https://github.com/naporin0624/ndi-to-sharedtexture/commit/efba0c2b6a03e3c0c11b9429f31c8a571995e05b))
* **gui:** let tray Quit through the close interceptor (quitting flag) ([41a7872](https://github.com/naporin0624/ndi-to-sharedtexture/commit/41a78729321dd8816e9bf17eca783f9b894380fa))
* **gui:** newline-separated info + auto-fit window height (no idle gap) ([82136d7](https://github.com/naporin0624/ndi-to-sharedtexture/commit/82136d75f2b44098ec5f7bcb8e99125df4ed2ae0))
* **gui:** pin refresh icon right via right_to_left, dropdown fills the rest ([0f4bf64](https://github.com/naporin0624/ndi-to-sharedtexture/commit/0f4bf64edc9f2de0018c500fa898b97cf10596f9))
* **gui:** reserve icon slot so dropdown+refresh always fit the window width ([f888df1](https://github.com/naporin0624/ndi-to-sharedtexture/commit/f888df1c2a17315bacb63e4c9fb06edb1242f508))
* **gui:** surface unexpected discovery-thread exit instead of hanging ([1a4e0e0](https://github.com/naporin0624/ndi-to-sharedtexture/commit/1a4e0e01a074a8f1d450a2435dfb88a022dbcd50))
* **gui:** widen window + trim combo so the refresh icon isn't clipped ([b3a34c7](https://github.com/naporin0624/ndi-to-sharedtexture/commit/b3a34c7851846694326960d74746b46ee648b664))
* **gui:** wrap info line + always-on status row to prevent layout shift ([c6dd801](https://github.com/naporin0624/ndi-to-sharedtexture/commit/c6dd8018550e495db5509d2d3fb5876c3519fe52))
* harden VideoFrame::data and silence dead_code warnings ([90e829f](https://github.com/naporin0624/ndi-to-sharedtexture/commit/90e829f7ef1e130e8c135890dfbdbc0e92cc87dd))
