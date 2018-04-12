/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//TODO: URL here

use canvas_traits::webgpu as w;
use dom::bindings::reflector::Reflector;
use dom::bindings::root::LayoutDom;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGPUSwapChain {
    reflector_: Reflector,
    id: w::SwapchainId,
    #[ignore_malloc_size_of = "Defined in ipc-channel"]
    sender: w::WebGPUMainChan,
}

impl WebGPUSwapChain {
    pub fn id(&self) -> w::SwapchainId {
        self.id
    }
}

impl Drop for WebGPUSwapChain {
    fn drop(&mut self) {
        //TODO
    }
}
