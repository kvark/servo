/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use super::{FrameHandler, ResourceHub};

use canvas_traits::{hal, webgpu as w};
use webgpu_thread::WebGPUThread;

use webrender_api;
use webrender::ExternalImageHandler;


 /// WebGPU Threading API entry point that lives in the constellation.
pub struct WebGPUThreads(w::WebGPUSender<w::Message>);

impl WebGPUThreads {
    /// Creates a new WebGPUThreads object
    pub fn new<B: hal::Backend>(
        webrender_api_sender: webrender_api::RenderApiSender,
        adapter: hal::Adapter<B>,
    ) -> (Option<Self>, webrender_api::IdNamespace, Box<ExternalImageHandler>) {
        let rehub = ResourceHub::<B>::new();
        let (handler, frame_sender) = FrameHandler::new();
        // This implementation creates a single `WebGPUThread` for all the pipelines.
        let (channel, namespace) = WebGPUThread::start(
            webrender_api_sender, frame_sender, adapter, rehub
        );
        (Some(WebGPUThreads(channel)), namespace, handler)
    }

    /// Gets the WebGPUThread handle for each script pipeline.
    pub fn pipeline(&self) -> w::WebGPUPipeline {
        // This mode creates a single thread, so the existing WebGpuChan is just cloned.
        w::WebGPUPipeline::new(self.0.clone())
    }

    /// Sends a exit message to close the WebGPUThreads and release all WebGPU contexts.
    pub fn exit(&self) -> Result<(), &'static str> {
        self.0.send(w::Message::Exit).map_err(|_| "Failed to send Exit message")
    }
}
