/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::{hal, webgpu as w};
use self::hal::{PhysicalDevice, QueueFamily};

use std::{iter, thread};
use std::sync::{mpsc, Arc, Mutex};

use webgpu_mode::{FrameMessage, LazyVec, ResourceHub, Swapchain, SwapchainShareMode};
/// WebGPU Threading API entry point that lives in the constellation.
/// It allows to get a WebGPUThread handle for each script pipeline.
pub use webgpu_mode::WebGPUThreads;

use euclid::Size2D;
use webrender_api as wrapi;


pub struct WebGPUThread<B: hal::Backend> {
    /// Channel used to generate/update or delete `wrapi::ImageKey`s.
    webrender_api: wrapi::RenderApi,
    frame_sender: mpsc::Sender<(wrapi::ExternalImageId, FrameMessage<B>)>,
    share_mode: SwapchainShareMode,
    next_external_image_id: wrapi::ExternalImageId,
    adapter: hal::Adapter<B>,
    _rehub: Arc<ResourceHub<B>>,
    gpus: LazyVec<Arc<hal::Gpu<B>>>,
    queues: LazyVec<hal::CommandQueue<B, hal::General>>,
    swapchains: LazyVec<Arc<Mutex<Swapchain<B>>>>,
}

impl<B: hal::Backend> WebGPUThread<B> {
    /// Creates a new `WebGPUThread` and returns a Sender to
    /// communicate with it.
    pub fn start(
        webrender_api_sender: wrapi::RenderApiSender,
        frame_sender: mpsc::Sender<(wrapi::ExternalImageId, FrameMessage<B>)>,
        adapter: hal::Adapter<B>,
        rehub: Arc<ResourceHub<B>>,
    ) -> (w::WebGPUSender<w::Message>, wrapi::IdNamespace) {
        let webrender_api = webrender_api_sender.create_api();
        let namespace = webrender_api.get_namespace_id();
        let (sender, receiver) = w::webgpu_channel::<w::Message>().unwrap();
        let result = sender.clone();
        thread::Builder::new().name("WebGPUThread".to_owned()).spawn(move || {
            //let instance = backend::Instance::create("Servo", 1);
            let mut renderer: Self = WebGPUThread {
                webrender_api,
                frame_sender,
                share_mode: SwapchainShareMode::Readback,
                next_external_image_id: wrapi::ExternalImageId(0x1000),
                adapter,
                _rehub: rehub,
                gpus: LazyVec::new(),
                queues: LazyVec::new(),
                swapchains: LazyVec::new(),
            };
            let webgpu_chan = sender;
            loop {
                //renderer.process_pool_commands();
                let msg = receiver.recv().unwrap();
                let exit = renderer.handle_msg(msg, &webgpu_chan);
                if exit {
                    return;
                }
            }
        }).expect("Thread spawning failed");

        (result, namespace)
    }

    /// Handles a generic Message message
    fn handle_msg(&mut self, msg: w::Message, _webgpu_chan: &w::WebGPUMainChan) -> bool {
        debug!("got message {:?}", msg);
        match msg {
            w::Message::Init { result } => {
                let info = self.init();
                result.send(info).unwrap();
            }
            w::Message::Exit => {
                return true;
            }
            w::Message::CreateDevice { result } => {
                let info = self.create_device();
                result.send(info).unwrap();
            }
            w::Message::CreateSwapChain { device, size, result } => {
                let info = self.create_swapchain(device, size);
                result.send(info).unwrap();
            }
            w::Message::AcquireFrame { device, swapchain, result } => {
                let info = self.acquire_frame(device, swapchain);
                result.send(info).unwrap();
            }
            w::Message::Present { queue, swapchain } => {
                self.present(queue, swapchain);
            }
        }
        false
    }

    fn init(&mut self) -> Result<w::InstanceInfo, String> {
        Ok(w::InstanceInfo {
            adapter_info: self.adapter.info.clone(),
            features: self.adapter.physical_device.features(),
            limits: self.adapter.physical_device.limits(),
        })
    }

    fn create_device(&mut self) -> Result<w::DeviceInfo, String> {
        let priorities = [1.0];
        let family = &self.adapter.queue_families[0];
        let families = [(family, &priorities[..])];
        let mut gpu = self.adapter.physical_device
            .open(&families)
            .map_err(|e| e.to_string())?;
        let queue = gpu.queues
            .take(family.id())
            .unwrap()
            .queues
            .remove(0);
        Ok(w::DeviceInfo {
            id: self.gpus.push(Arc::new(gpu)),
            queue_id: self.queues.push(queue),
        })
    }

    fn create_swapchain(
        &mut self, dev_id: w::DeviceId, size: Size2D<u32>,
    ) -> Result<w::SwapChainInfo, String> {
        let num_frames = 3;
        let format = hal::format::Format::Rgba8Srgb;
        let image_key = self.webrender_api.generate_image_key();

        match self.share_mode {
            SwapchainShareMode::SharedTexture => {} //TODO
            SwapchainShareMode::Readback => {
                let desc = wrapi::ImageDescriptor {
                    format: wrapi::ImageFormat::BGRA8,
                    width: size.width,
                    height: size.height,
                    stride: None,
                    offset: 0,
                    is_opaque: true,
                };

                let data = if false { // raw pixels?
                    let pixels = (0..size.width*size.height*4).map(|_| 0u8).collect();
                    wrapi::ImageData::Raw(Arc::new(pixels))
                } else {
                    wrapi::ImageData::External(wrapi::ExternalImageData {
                        id: self.next_external_image_id,
                        channel_index: 0,
                        image_type: wrapi::ExternalImageType::Buffer,
                    })
                };

                let mut updates = wrapi::ResourceUpdates::new();
                updates.add_image(image_key, desc, data, None);
                self.webrender_api.update_resources(updates);
            }
        }

        let dev = &self.gpus[dev_id].device;
        let queue_family = self.adapter.queue_families[0].id();
        let memory_types = self.adapter.physical_device
            .memory_properties()
            .memory_types;
        let swapchain = Swapchain::new(
            self.share_mode,
            size, num_frames, format,
            dev, queue_family, &memory_types,
        );
        let sc_arc = Arc::new(Mutex::new(swapchain));
        let id = self.swapchains.push(sc_arc.clone());

        let message = FrameMessage::Add(
            self.gpus[dev_id].clone(),
            sc_arc
        );
        self.frame_sender
            .send((self.next_external_image_id, message))
            .unwrap();

        let textures = (0 .. num_frames)
            .map(|i| w::TextureInfo {
                id: w::TextureId::Swapchain(id, i),
            })
            .collect();

        Ok(w::SwapChainInfo {
            id,
            textures,
            image_key,
        })
    }

    fn acquire_frame(
        &mut self, dev_id: w::DeviceId, swapchain_id: w::SwapChainId
    ) -> w::TextureInfo {
        let dev = &self.gpus[dev_id].device;
        let mut swapchain = self.swapchains[swapchain_id].lock().unwrap();
        let index = swapchain.acquire_frame(dev);

        w::TextureInfo {
            id: w::TextureId::Swapchain(swapchain_id, index),
        }
    }

    fn present(&mut self, queue_id: w::QueueId, swapchain_id: w::SwapChainId) {
        let queue = &mut self.queues[queue_id];
        let mut swapchain = self.swapchains[swapchain_id].lock().unwrap();
        let (submit, fence) = swapchain.present();
        let submission = hal::Submission::new()
            .submit(iter::once(submit));
        queue.submit(submission, Some(fence));
    }
}
