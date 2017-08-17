/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::*;
use std::thread;
use webgpu::{backend, QueueType};
use webgpu::gpu::{Adapter, Instance, QueueFamily};


pub struct WebGpuThread {
    /// Id generator for new WebGpuContexts.
    next_webgpu_id: usize,
    instance: backend::Instance,
    adapters: Vec<backend::Adapter>,
}

impl WebGpuThread {
    fn new() -> Self {
        let instance = backend::Instance::create();
        let adapters = instance.enumerate_adapters();
        WebGpuThread {
            next_webgpu_id: 0,
            instance,
            adapters,
        }
    }

    /// Creates a new `WebGpuThread` and returns a Sender to
    /// communicate with it.
    pub fn start() -> WebGpuSender<WebGpuMsg> {
        let (sender, receiver) = webgpu_channel::<WebGpuMsg>().unwrap();
        let result = sender.clone();
        thread::Builder::new().name("WebGpuThread".to_owned()).spawn(move || {
            let mut renderer = WebGpuThread::new();
            let webgpu_chan = sender;
            loop {
                let msg = receiver.recv().unwrap();
                let exit = renderer.handle_msg(msg, &webgpu_chan);
                if exit {
                    return;
                }
            }
        }).expect("Thread spawning failed");

        result
    }

    /// Handles a generic WebGpuMsg message
    #[inline]
    fn handle_msg(&mut self, msg: WebGpuMsg, webgpu_chan: &WebGpuChan) -> bool {
        match msg {
            WebGpuMsg::CreateContext(sender) => {
                let init = self
                    .create_webgpu_context()
                    .map(|(id, adapters)| WebGpuInit {
                        sender: WebGpuMsgSender::new(id, webgpu_chan.clone()),
                        adapters,
                    });
                sender.send(init).unwrap();
            }
            WebGpuMsg::Exit => {
                return true;
            }
        }

        false
    }

    /// Creates a new WebGpuContext
    fn create_webgpu_context(&mut self
    ) -> Result<(WebGpuContextId, Vec<AdapterInfo>), String>
    {
        let adapters = self.adapters
            .iter()
            .map(|ad| {
                let queue_families = ad
                    .get_queue_families()
                    .map(|family| {
                        let ty = if family.supports_graphics() {
                            QueueType::Graphics
                        } else if family.supports_compute() {
                            QueueType::Compute
                        } else {
                            QueueType::Transfer
                        };
                        QueueInfo {
                            ty,
                            count: family.num_queues() as u8,
                        }
                    })
                    .collect();
                AdapterInfo {
                    info: ad.get_info().clone(),
                    queue_families,
                }
            })
            .collect();
        let id = WebGpuContextId(self.next_webgpu_id);
        self.next_webgpu_id += 1;
        Ok((id, adapters))
    }
}

/// WebGPU Threading API entry point that lives in the constellation.
pub struct WebGpuThreads(WebGpuSender<WebGpuMsg>);

impl WebGpuThreads {
    /// Creates a new WebGpuThreads object
    pub fn new() -> Self {
        // This implementation creates a single `WebGpuThread` for all the pipelines.
        WebGpuThreads(WebGpuThread::start())
    }

    /// Gets the WebGpuThread handle for each script pipeline.
    pub fn pipeline(&self) -> WebGpuPipeline {
        // This mode creates a single thread, so the existing WebGpuChan is just cloned.
        WebGpuPipeline(self.0.clone())
    }

    /// Sends a exit message to close the WebGpuThreads and release all WebGpuContexts.
    pub fn exit(&self) -> Result<(), &'static str> {
        self.0.send(WebGpuMsg::Exit).map_err(|_| "Failed to send Exit message")
    }
}
