/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//TODO: URL here

use canvas_traits::webgpu as w;
//use dom::bindings::codegen::Bindings::WebGPUSwapChainBinding as binding;
use dom::bindings::reflector::{DomObject, Reflector};
use dom::bindings::root::DomRoot;
use dom::webgputexture::WebGPUTexture;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGPUSwapChain {
    reflector_: Reflector,
    id: (w::DeviceId, w::SwapChainId),
    #[ignore_malloc_size_of = "Defined in ipc-channel"]
    sender: w::WebGPUMainChan,
}

impl WebGPUSwapChain {
    #[allow(unrooted_must_root)]
    pub fn new_internal(
        device: w::DeviceId, id: w::SwapChainId, sender: w::WebGPUMainChan
    ) -> Self {
        WebGPUSwapChain {
            reflector_: Reflector::new(),
            id: (device, id),
            sender,
        }
    }

    pub fn _id(&self) -> w::SwapChainId {
        self.id.1
    }
}

impl Drop for WebGPUSwapChain {
    fn drop(&mut self) {
        //TODO
    }
}

impl WebGPUSwapChain {
    pub fn GetNextTexture(&self) -> DomRoot<WebGPUTexture> {
        let (sender, receiver) = w::webgpu_channel().unwrap();
        let msg = w::Message::AcquireFrame {
            device: self.id.0,
            swapchain: self.id.1,
            result: sender,
        };
        self.sender.send(msg).unwrap();

        let info = receiver.recv().unwrap();
        WebGPUTexture::new(self.global().as_window(), info)
    }

    pub fn Present(&self) {
        //TODO
    }
}
