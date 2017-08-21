/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::webgpu::{WebGpuCommand, WebGpuCommandChan};
use dom::bindings::codegen::Bindings::WebGpuCommandPoolBinding as binding;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom_struct::dom_struct;


#[dom_struct]
pub struct WebGpuCommandPool {
    reflector_: Reflector,
    #[ignore_heap_size_of = "Channels are hard"]
    sender: WebGpuCommandChan,
}

impl WebGpuCommandPool {
    pub fn new(global: &GlobalScope, sender: WebGpuCommandChan) -> Root<Self> {
        let obj = box WebGpuCommandPool {
            reflector_: Reflector::new(),
            sender,
        };
        reflect_dom_object(obj, global, binding::Wrap)
    }
}

impl binding::WebGpuCommandPoolMethods for WebGpuCommandPool {
    fn Reset(&self) {
        let msg = WebGpuCommand::Reset;
        self.sender.send(msg).unwrap()
    }
}
