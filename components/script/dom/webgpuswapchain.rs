/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//TODO: URL here

use canvas_traits::webgpu as w;
use dom::bindings::codegen::Bindings::WebGPUSwapChainBinding as binding;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::bindings::root::DomRoot;
use dom::window::Window;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGPUSwapChain {
    reflector_: Reflector,
    id: w::SwapchainId,
    #[ignore_malloc_size_of = "Defined in ipc-channel"]
    sender: w::WebGPUMainChan,
}

impl WebGPUSwapChain {
    #[allow(unrooted_must_root)]
    pub fn new_internal(id: w::SwapchainId, sender: w::WebGPUMainChan) -> Self {
        WebGPUSwapChain {
            reflector_: Reflector::new(),
            id,
            sender,
        }
    }

    #[allow(unrooted_must_root)]
    pub fn new(
        window: &Window, id: w::SwapchainId, sender: w::WebGPUMainChan,
    ) -> DomRoot<Self> {
        let object = Self::new_internal(id, sender);
        reflect_dom_object(Box::new(object), window, binding::Wrap)
    }

    pub fn id(&self) -> w::SwapchainId {
        self.id
    }
}

impl Drop for WebGPUSwapChain {
    fn drop(&mut self) {
        //TODO
    }
}
