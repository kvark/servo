/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::{hal, webgpu as w};
/*use webgpu::hal::{self,
    Adapter, DescriptorPool, Device, Instance, QueueFamily,
    RawCommandBuffer, RawCommandPool, RawCommandQueue,
};*/

use std::thread;
//use std::cmp::Ordering;
//use std::collections::HashMap;
use std::sync::{Arc};

use webgpu_mode::{LazyVec, ResourceHub};
/// WebGL Threading API entry point that lives in the constellation.
/// It allows to get a WebGPUThread handle for each script pipeline.
pub use webgpu_mode::WebGPUThreads;

//use euclid::Size2D;
use webrender_api as wrapi;


pub struct WebGPUThread<B: hal::Backend> {
    /// Channel used to generate/update or delete `wrapi::ImageKey`s.
    webrender_api: wrapi::RenderApi,
    //present_chan: w::WebGPUPresentChan,
    //adapters: Vec<B::Adapter>,
    //memories: LazyVec<Memory<B>>,
    rehub: Arc<ResourceHub<B>>,
    //command_pools: LazyVec<CommandPoolHandle<B>>,
}

impl<B: hal::Backend> WebGPUThread<B> {
    /// Creates a new `WebGPUThread` and returns a Sender to
    /// communicate with it.
    pub fn start(
        webrender_api_sender: wrapi::RenderApiSender,
        //present_chan: w::WebGPUPresentChan,
        rehub: Arc<ResourceHub<B>>,
    ) -> (w::WebGPUSender<w::WebGPUMsg>, wrapi::IdNamespace) {
        let webrender_api = webrender_api_sender.create_api();
        let namespace = webrender_api.get_namespace_id();
        let (sender, receiver) = w::webgpu_channel::<w::WebGPUMsg>().unwrap();
        let result = sender.clone();
        thread::Builder::new().name("WebGPUThread".to_owned()).spawn(move || {
            //let instance = backend::Instance::create("Servo", 1);
            let mut renderer: Self = WebGPUThread {
                webrender_api,
                //present_chan,
                //adapters: instance.enumerate_adapters(),
                //memories: LazyVec::new(),
                rehub,
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

    /// Handles a generic WebGPUMsg message
    fn handle_msg(&mut self, msg: w::WebGPUMsg, webgpu_chan: &w::WebGPUMainChan) -> bool {
        debug!("got message {:?}", msg);
        match msg {
            w::WebGPUMsg::CreateContext { .. } => {
                /*let info = self
                    .create_context(size, external_image_id)
                    .map(|(presenter, adapters, image_key)| w::ContextInfo {
                        presenter,
                        adapters,
                        sender: webgpu_chan.clone(),
                        image_key,
                    });
                result.send(info).unwrap();*/
            }
            w::WebGPUMsg::Exit => {
                return true;
            }
        }

        false
    }
}
