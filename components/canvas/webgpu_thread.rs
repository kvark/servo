/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::{hal, webgpu as w};
use canvas_traits::hal::PhysicalDevice;

use std::thread;
use std::sync::{Arc};

use webgpu_mode::{LazyVec, ResourceHub, Swapchain};
/// WebGL Threading API entry point that lives in the constellation.
/// It allows to get a WebGPUThread handle for each script pipeline.
pub use webgpu_mode::WebGPUThreads;

use euclid::Size2D;
use webrender_api as wrapi;


pub struct WebGPUThread<B: hal::Backend> {
    /// Channel used to generate/update or delete `wrapi::ImageKey`s.
    webrender_api: wrapi::RenderApi,
    //present_chan: w::WebGPUPresentChan,
    //adapters: Vec<B::Adapter>,
    //memories: LazyVec<Memory<B>>,
    adapter: hal::Adapter<B>,
    rehub: Arc<ResourceHub<B>>,
    swapchains: LazyVec<Swapchain<B>>,
    //command_pools: LazyVec<CommandPoolHandle<B>>,
}

impl<B: hal::Backend> WebGPUThread<B> {
    /// Creates a new `WebGPUThread` and returns a Sender to
    /// communicate with it.
    pub fn start(
        webrender_api_sender: wrapi::RenderApiSender,
        //present_chan: w::WebGPUPresentChan,
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
                //present_chan,
                //adapters: instance.enumerate_adapters(),
                //memories: LazyVec::new(),
                adapter,
                rehub,
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
            w::Message::CreateDevice { result } => {
                let info = self.create_device();
                result.send(info).unwrap();
            }
            w::Message::CreateSwapChain { device, size, result } => {
                let info = self.create_swapchain(device, size);
                result.send(info).unwrap();
            }
            w::Message::Exit => {
                return true;
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
        let families = vec![(&self.adapter.queue_families[0], vec![1.0])];
        let gpu = self.adapter.physical_device
            .open(families)
            .map_err(|e| e.to_string())?;
        let id = self.rehub.gpus.write().unwrap().push(gpu);
        Ok(w::DeviceInfo {
            id,
        })
    }

    fn create_swapchain(
        &mut self, device: w::DeviceId, size: Size2D<u32>
    ) -> Result<w::SwapChainInfo, String> {
        let image_key = self.webrender_api.generate_image_key();
        let dev = &self.rehub.gpus.read().unwrap()[device].device;
        let swapchain = Swapchain::new(dev, size);
        let id = self.swapchains.push(swapchain);
        Ok(w::SwapChainInfo {
            id,
            image_key,
        })
    }
}
