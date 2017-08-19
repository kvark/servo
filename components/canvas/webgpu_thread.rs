/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu as w;
use webgpu::{backend, QueueType};
use webgpu::gpu::{self, Adapter, Factory, Instance, QueueFamily};

use euclid::Size2D;

use std::{ops, thread};


type BackendDevice = gpu::Device<backend::Resources, backend::Factory, backend::CommandQueue>;

struct LazyVec<T> {
    inner: Vec<Option<T>>,
}

impl<T> LazyVec<T> {
    fn new() -> Self {
        LazyVec {
            inner: Vec::new(),
        }
    }

    fn push(&mut self, value: T) -> usize {
        let id = self.inner.len(); //TODO: recycle
        self.inner.push(Some(value));
        id
    }
}

impl<T> ops::Index<usize> for LazyVec<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        self.inner[index].as_ref().unwrap()
    }
}
impl<T> ops::IndexMut<usize> for LazyVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        self.inner[index].as_mut().unwrap()
    }
}


pub struct WebGpuThread {
    /// Id generator for new WebGpuContexts.
    next_webgpu_id: usize,
    _instance: backend::Instance,
    adapters: Vec<backend::Adapter>,
    devices: LazyVec<BackendDevice>,
    heaps: LazyVec<<backend::Resources as gpu::Resources>::Heap>,
    images: LazyVec<<backend::Resources as gpu::Resources>::Image>,
}

impl WebGpuThread {
    fn new() -> Self {
        let instance = backend::Instance::create();
        let adapters = instance.enumerate_adapters();
        WebGpuThread {
            next_webgpu_id: 0,
            _instance: instance,
            adapters,
            devices: LazyVec::new(),
            heaps: LazyVec::new(),
            images: LazyVec::new(),
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
                    id: self.devices.push(device) as w::DeviceId,
                };
                result.send(info).unwrap();
            }
            w::WebGpuMsg::BuildSwapchain { device_id, size, result } => {
                let swapchain = self.build_swapchain(device_id, size);
                result.send(swapchain).unwrap();
            }
            w::WebGpuMsg::Present(image_id) => {
                //TODO
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

    fn build_swapchain(&mut self, device_id: w::DeviceId, size: Size2D<u32>) -> w::SwapchainInfo {
        let device = &mut self.devices[device_id as usize];
        let image_store = &mut self.images;
        let num_frames = 3; //TODO
        let alignment = 0x3F;
        let bytes_per_image = 4 *
            (size.width as u64 | alignment + 1)*
            (size.height as u64| alignment + 1);

        let heap = {
            let heap_type = device.heap_types
                .iter()
                .find(|ht| ht.properties.contains(gpu::memory::DEVICE_LOCAL))
                .unwrap();
            device.factory.create_heap(
                heap_type,
                gpu::factory::ResourceHeapType::Images,
                num_frames * bytes_per_image,
            ).unwrap()
        };

        let images = (0 .. num_frames).map(|i| {
            let unbound_image = device.factory.create_image(
                gpu::image::Kind::D2(
                    size.width as gpu::image::Size,
                    size.height as gpu::image::Size,
                    gpu::image::AaMode::Single,
                ),
                1,
                <gpu::format::Srgba8 as gpu::format::Formatted>::get_format(),
                gpu::image::COLOR_ATTACHMENT | gpu::image::TRANSFER_SRC,
            ).unwrap();
            let image = device.factory.bind_image_memory(
                &heap,
                i * bytes_per_image,
                unbound_image,
            ).unwrap();
            image_store.push(image) as w::ImageId
        }).collect();

        w::SwapchainInfo {
            heap_id: self.heaps.push(heap) as w::HeapId,
            images,
        }
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
