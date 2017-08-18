/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu as w;
use std::thread;
use webgpu::{backend, QueueType};
use webgpu::gpu::{Adapter, Device, Instance, QueueFamily};

type BackendDevice = Device<backend::Resources, backend::Factory, backend::CommandQueue>;

pub struct WebGpuThread {
    /// Id generator for new WebGpuContexts.
    next_webgpu_id: usize,
    _instance: backend::Instance,
    adapters: Vec<backend::Adapter>,
    devices: Vec<Option<BackendDevice>>,
}

impl WebGpuThread {
    fn new() -> Self {
        let instance = backend::Instance::create();
        let adapters = instance.enumerate_adapters();
        WebGpuThread {
            next_webgpu_id: 0,
            _instance: instance,
            adapters,
            devices: Vec::new(),
        }
    }

    /// Creates a new `WebGpuThread` and returns a Sender to
    /// communicate with it.
    pub fn start() -> w::WebGpuSender<w::WebGpuMsg> {
        let (sender, receiver) = w::webgpu_channel::<w::WebGpuMsg>().unwrap();
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
    fn handle_msg(&mut self, msg: w::WebGpuMsg, webgpu_chan: &w::WebGpuChan) -> bool {
        match msg {
            w::WebGpuMsg::CreateContext(sender) => {
                let init = self
                    .create_webgpu_context()
                    .map(|(id, adapters)| w::ContextInfo {
                        sender: w::WebGpuMsgSender::new(id, webgpu_chan.clone()),
                        adapters,
                    });
                sender.send(init).unwrap();
            }
            w::WebGpuMsg::OpenAdapter { adapter_id, queue_families, result } => {
                let adapter = &mut self.adapters[adapter_id as usize];
                let families = queue_families
                    .iter()
                    .map(|&(id, count)| {
                        let family = adapter
                            .get_queue_families()
                            .nth(id as usize)
                            .unwrap();
                        (family, count as u32)
                    });
                let device = adapter.open(families);
                let info = w::DeviceInfo {
                    id: self.devices.len() as w::DeviceId,
                };
                self.devices.push(Some(device)); //TODO: recycle
                result.send(info).unwrap();
            }
            w::WebGpuMsg::Exit => {
                return true;
            }
        }

        false
    }

    /// Creates a new WebGpuContext
    fn create_webgpu_context(&mut self
    ) -> Result<(w::WebGpuContextId, Vec<w::AdapterInfo>), String>
    {
        let adapters = self.adapters
            .iter()
            .enumerate()
            .map(|(aid, ad)| {
                let queue_families = ad
                    .get_queue_families()
                    .enumerate()
                    .map(|(qid, family)| {
                        let ty = if family.supports_graphics() {
                            QueueType::Graphics
                        } else if family.supports_compute() {
                            QueueType::Compute
                        } else {
                            QueueType::Transfer
                        };
                        w::QueueFamilyInfo {
                            ty,
                            count: family.num_queues() as u8,
                            original_id: qid as w::QueueFamilyId,
                        }
                    })
                    .collect();
                w::AdapterInfo {
                    info: ad.get_info().clone(),
                    queue_families,
                    original_id: aid as w::AdapterId,
                }
            })
            .collect();
        let id = w::WebGpuContextId(self.next_webgpu_id);
        self.next_webgpu_id += 1;
        Ok((id, adapters))
    }
}

/// WebGPU Threading API entry point that lives in the constellation.
pub struct WebGpuThreads(w::WebGpuSender<w::WebGpuMsg>);

impl WebGpuThreads {
    /// Creates a new WebGpuThreads object
    pub fn new() -> Self {
        // This implementation creates a single `WebGpuThread` for all the pipelines.
        WebGpuThreads(WebGpuThread::start())
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
