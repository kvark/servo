/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use super::ResourceHub;

use canvas_traits::{hal, webgpu as w};
use webgpu_thread::WebGPUThread;

use webrender_api;


 /// WebGPU Threading API entry point that lives in the constellation.
pub struct WebGPUThreads(w::WebGPUSender<w::WebGPUMsg>);

impl WebGPUThreads {
    /// Creates a new WebGPUThreads object
    pub fn new<B: hal::Backend>(
        webrender_api_sender: webrender_api::RenderApiSender,
    ) -> (Option<Self>, webrender_api::IdNamespace) {
        let rehub = ResourceHub::<B>::new();
        // This implementation creates a single `WebGPUThread` for all the pipelines.
        let (channel, namespace) = WebGPUThread::start(webrender_api_sender, rehub);
        (Some(WebGPUThreads(channel)), namespace)
    }

    /// Gets the WebGPUThread handle for each script pipeline.
    pub fn pipeline(&self) -> w::WebGPUPipeline {
        // This mode creates a single thread, so the existing WebGpuChan is just cloned.
        w::WebGPUPipeline::new(self.0.clone())
    }

    /// Sends a exit message to close the WebGPUThreads and release all WebGPU contexts.
    pub fn exit(&self) -> Result<(), &'static str> {
        self.0.send(w::WebGPUMsg::Exit).map_err(|_| "Failed to send Exit message")
    }
}
