/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu as w;
use ::webgpu_thread::WebGpuThread;
use webrender_api;

 /// WebGPU Threading API entry point that lives in the constellation.
pub struct WebGpuThreads(w::WebGpuSender<w::WebGpuMsg>);

impl WebGpuThreads {
    /// Creates a new WebGpuThreads object
    pub fn new(
        webrender_api_sender: webrender_api::RenderApiSender,
    ) -> Self {
        // This implementation creates a single `WebGpuThread` for all the pipelines.
        WebGpuThreads(WebGpuThread::start(webrender_api_sender))
    }

    /// Gets the WebGpuThread handle for each script pipeline.
    pub fn pipeline(&self) -> w::WebGpuPipeline {
        // This mode creates a single thread, so the existing WebGpuChan is just cloned.
        w::WebGpuPipeline(self.0.clone())
    }

    /// Sends a exit message to close the WebGpuThreads and release all WebGpuContexts.
    pub fn exit(&self) -> Result<(), &'static str> {
        self.0.send(w::WebGpuMsg::Exit).map_err(|_| "Failed to send Exit message")
    }
}
